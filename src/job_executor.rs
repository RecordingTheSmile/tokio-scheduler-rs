use std::future::Future;

use parking_lot::RwLock;
use std::pin::Pin;
use std::sync::Arc;

use crate::job_hook::{JobHook, JobHookReturn};
use crate::job_storage::JobStorage;

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
    Tz: chrono::TimeZone + Send + Sync,
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
            loop {
                let should_next = *should_next.read();

                if !should_next {
                    let _ = shutdown_sender.send(());
                    break;
                }
                let should_exec = match storage.get_all_should_execute_jobs().await {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let should_exec = match storage
                    .get_jobs_by_name(&should_exec.into_iter().map(|v| (v.0, v.1, v.2)).collect())
                    .await
                {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                for (job, name, id, args) in should_exec {
                    let hook = hook.to_owned();
                    let storage = storage.to_owned();
                    let handle = tokio::spawn(async move {
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
                                        log::error!(
                                            "[Tokio-Scheduler-Rs] SchedulerError: {:#?}",
                                            e
                                        );
                                    }
                                };
                                false
                            }
                            _ => true,
                        };
                        if !should_execute {
                            return;
                        }
                        let (job_context, job_execute_result) = job.execute().await;
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
                                            .on_fail(
                                                &name,
                                                &id,
                                                &args,
                                                &je,
                                                job_context.get_retry_times(),
                                            )
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

                        let handle_action = if handle_action != JobHookReturn::RemoveJob
                            && handle_action != JobHookReturn::RetryJob
                        {
                            if job_context.is_delete_scheduled() {
                                JobHookReturn::RemoveJob
                            } else if job_context.is_retry_scheduled() {
                                JobHookReturn::RetryJob
                            } else {
                                handle_action
                            }
                        } else {
                            handle_action
                        };

                        let _ = match handle_action {
                            JobHookReturn::RemoveJob => {
                                match storage.delete_job(&id).await {
                                    Ok(_) => (),
                                    Err(e) => {
                                        log::error!(
                                            "[Tokio-Scheduler-Rs] SchedulerError: {:#?}",
                                            e
                                        );
                                    }
                                };
                            }
                            JobHookReturn::RetryJob => {
                                match storage
                                    .add_retry_job(&name, &args, job_context.get_retry_times() + 1)
                                    .await
                                {
                                    Ok(_) => (),
                                    Err(e) => {
                                        log::error!(
                                            "[Tokio-Scheduler-Rs] SchedulerError: {:#?}",
                                            e
                                        );
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

                tokio::time::sleep(std::time::Duration::from_secs(polling_time)).await;
            }
        })
    }

    /// Gracefully stop all the pending jobs and shut down the `JobExecutor`
    fn stop(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        *self.should_next.write() = false;
        let mut shutdown_channel = self.shutdown_channel.subscribe();
        let tasks = self.tasks.to_owned();
        let timeout = self.stop_timeout;
        Box::pin(async move {
            let _ = shutdown_channel.recv().await;
            for i in tasks.read().iter() {
                let timeout_fut = tokio::time::sleep(std::time::Duration::from_secs(timeout));
                let wait_fut = async {
                    loop {
                        if i.is_finished() {
                            break;
                        } else {
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        }
                    }
                };
                tokio::select! {
                    _ = timeout_fut => {
                        i.abort();
                    },
                    _ = wait_fut => {

                    }
                }
            }
        })
    }
}
