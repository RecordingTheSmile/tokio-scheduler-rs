use std::future::Future;

use parking_lot::RwLock;
use serde_json::Value;
use std::pin::Pin;
use std::sync::Arc;

use crate::job_hook::{JobHook, JobHookReturn};
use crate::job_storage::JobStorage;
use crate::WillExecuteJobFuture;

use tokio::task::JoinHandle;

/// `JobExecutor` is used to execute jobs
pub trait JobExecutor: Send + Sync {
    /// start job execution
    fn start(&self) -> JoinHandle<()>;
    /// stop job execution and wait for all running jobs to complete
    fn stop(&self) -> Pin<Box<dyn Future<Output = ()>>>;
}

/// Default implementation for `JobExecutor`
pub struct DefaultJobExecutor<Tz>
where
    Tz: chrono::TimeZone + Send + Sync,
    Tz::Offset: Send + Sync,
{
    jobs: Arc<dyn JobStorage<Tz>>,
    tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
    shutdown_channel: tokio::sync::broadcast::Sender<()>,
    should_next: Arc<RwLock<bool>>,
    polling_time: u64,
    hook: Arc<Option<Box<dyn JobHook>>>,
    stop_timeout: u64,
}

impl<Tz> DefaultJobExecutor<Tz>
where
    Tz: chrono::TimeZone + Send + Sync + 'static,
    Tz::Offset: Send + Sync,
{
    /// Create a new Default JobExecutor.
    ///
    /// # Arguments
    /// `jobs`: A `JobStorage` trait
    ///
    /// `polling_time`: How often should `JobExecutor` query the `JobStorage`.
    ///
    /// `hooks`: A `JobHook`, can be None.
    ///
    /// `stop_timeout`: How long should force cancel a job when call `stop()` fn.
    pub fn new(
        jobs: Arc<dyn JobStorage<Tz>>,
        polling_time: Option<u64>,
        hooks: Option<Box<dyn JobHook>>,
        stop_timeout: u64,
    ) -> Self {
        let shutdown_chan = tokio::sync::broadcast::channel(1);
        Self {
            jobs,
            tasks: Arc::new(RwLock::new(vec![])),
            shutdown_channel: shutdown_chan.0,
            should_next: Arc::new(RwLock::new(true)),
            polling_time: polling_time.unwrap_or(5),
            hook: Arc::new(hooks),
            stop_timeout,
        }
    }

    #[tracing::instrument(skip_all)]
    async fn process_tasks(
        all_tasks: Vec<(WillExecuteJobFuture, String, String, Option<Value>)>,
        hook: Arc<Option<Box<dyn JobHook>>>,
        storage: Arc<dyn JobStorage<Tz>>,
        tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
    ) {
        for (job, name, id, args) in all_tasks {
            let hook = hook.to_owned();
            let storage = storage.to_owned();
            let handle = tokio::spawn(async move {
                // call job hook to judge if we execute this task or cancel/remove this task
                let handle_action = match &*hook {
                    Some(v) => v.on_execute(&name, &id, &args).await,
                    None => JobHookReturn::NoAction,
                };
                let should_execute = match handle_action {
                    JobHookReturn::CancelRunning => false,
                    JobHookReturn::RemoveJob => {
                        match storage.delete_job(&id).await {
                            Ok(_) => (),
                            Err(e) => {
                                tracing::error!("JobHookError delete job error: {:?}", e);
                            }
                        };
                        false
                    }
                    _ => true,
                };
                if !should_execute {
                    return;
                }
                let job_id = job.get_job_context().get_id().to_owned();
                tracing::debug!("Start to execute job: {}", job_id);
                let (job_context, job_execute_result) = job.execute().await;
                tracing::debug!("Job: {} executed, result: {:?}", job_id, job_execute_result);
                let handle_action = match &*hook {
                    Some(v) => {
                        let mut final_result = v
                            .on_complete(
                                &name,
                                &id,
                                &args,
                                &job_execute_result,
                                job_context.get_retry_times(),
                            )
                            .await;

                        match job_execute_result {
                            Ok(jo) => {
                                let success_result = v
                                    .on_success(
                                        &name,
                                        &id,
                                        &args,
                                        &jo,
                                        job_context.get_retry_times(),
                                    )
                                    .await;
                                if success_result != final_result {
                                    final_result = success_result;
                                }
                            }
                            Err(je) => {
                                let error_result = v
                                    .on_fail(&name, &id, &args, &je, job_context.get_retry_times())
                                    .await;
                                if error_result != final_result {
                                    final_result = error_result;
                                }
                            }
                        }
                        final_result
                    }
                    None => JobHookReturn::NoAction,
                };

                _ = match handle_action {
                    JobHookReturn::RemoveJob => {
                        match storage.delete_job(&id).await {
                            Ok(_) => (),
                            Err(e) => {
                                tracing::error!("JobHookError delete job error: {:?}", e);
                            }
                        };
                    }
                    JobHookReturn::RetryJob => {
                        match storage
                            .add_retry_job(&id, &name, &args, job_context.get_retry_times() + 1)
                            .await
                        {
                            Ok(_) => (),
                            Err(e) => {
                                tracing::error!("JobHookError retry job error: {:?}; Retry error when executing `storage.add_retry_job`", e);
                            }
                        };
                    }
                    _ => (),
                };
                ()
            });
            let mut task_vec = tasks.write();
            task_vec.push(handle);
        }
    }

    async fn perform_job_tasks(
        storage: Arc<dyn JobStorage<Tz>>,
        shutdown_sender: tokio::sync::broadcast::Sender<()>,
        should_next: Arc<RwLock<bool>>,
        tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
        polling_time: u64,
        hook: Arc<Option<Box<dyn JobHook>>>,
    ) {
        loop {
            // should we go next or exit (if recv the close signal) ?
            let should_next = *should_next.read();

            // exit if recv close signal
            if !should_next {
                _ = shutdown_sender.send(());
                break;
            }

            // remove all completed jobs
            {
                let tasks = tasks.to_owned();
                let mut tasks_remover = tasks.write();
                tasks_remover.retain(|task| !task.is_finished());
            }

            // get all tasks should be executed from storage
            let should_exec = match storage.get_all_should_execute_jobs().await {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("Get job from storage error: {:?}", e);
                    continue;
                }
            };

            // convert should_exec tasks to `WillExecuteJobFuture`
            let should_exec = match storage
                .get_jobs_by_name(
                    &should_exec
                        .into_iter()
                        .map(|v| (v.0, v.1, v.2, v.3))
                        .collect(),
                )
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("Get job from storage error: {:?}", e);
                    continue;
                }
            };

            Self::process_tasks(
                should_exec,
                hook.to_owned(),
                storage.to_owned(),
                tasks.to_owned(),
            )
            .await;

            // wait for next time execute
            tokio::time::sleep(std::time::Duration::from_secs(polling_time)).await;
        }
    }
}

impl<Tz> JobExecutor for DefaultJobExecutor<Tz>
where
    Tz: chrono::TimeZone + Send + Sync + 'static,
    Tz::Offset: Send + Sync,
{
    /// Start the job executor.
    fn start(&self) -> JoinHandle<()> {
        let storage = self.jobs.to_owned();
        let shutdown_sender = self.shutdown_channel.to_owned();
        let should_next = self.should_next.to_owned();
        let tasks = self.tasks.to_owned();
        let polling_time = self.polling_time;
        let hook = self.hook.to_owned();
        tokio::spawn(async move {
            Self::perform_job_tasks(
                storage,
                shutdown_sender,
                should_next,
                tasks,
                polling_time,
                hook,
            )
            .await;
        })
    }

    /// Gracefully stop all the pending jobs and shut down the `JobExecutor`
    fn stop(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        *self.should_next.write() = false;
        let mut shutdown_channel = self.shutdown_channel.subscribe();
        let tasks = self.tasks.to_owned();
        let timeout = self.stop_timeout;
        Box::pin(async move {
            // ensure the execute procedure is complete
            _ = shutdown_channel.recv().await;
            let mut task_reader = tasks.write();
            while let Some(task) = task_reader.pop() {
                let timeout_fut = tokio::time::sleep(std::time::Duration::from_secs(timeout));
                let wait_fut = async {
                    _ = task.await;
                };
                tokio::select! {
                    _ = timeout_fut => {
                        tracing::warn!("JobExecutor stop timeout, force cancel the job");
                    },
                    _ = wait_fut => {

                    }
                }
            }
        })
    }
}
