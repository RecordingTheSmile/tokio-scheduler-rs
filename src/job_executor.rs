use std::future::Future;

use std::pin::Pin;
use std::sync::Arc;

use crate::job_hook::{JobHook, JobHookReturn};
use crate::job_storage::JobStorage;

use tokio::sync::RwLock;
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
    hook: Arc<RwLock<Option<Box<dyn JobHook>>>>,
}

impl<Tz> DefaultJobExecutor<Tz>
where
    Tz: chrono::TimeZone + Send + Sync,
    Tz::Offset: Send + Sync,
{
    pub fn new(
        jobs: Arc<dyn JobStorage<Tz>>,
        polling_time: Option<u64>,
        hooks: Option<Box<dyn JobHook>>,
    ) -> Self {
        let shutdown_chan = tokio::sync::broadcast::channel(1);
        Self {
            jobs,
            tasks: Arc::new(RwLock::new(vec![])),
            shutdown_channel: shutdown_chan.0,
            should_next: Arc::new(RwLock::new(true)),
            polling_time: polling_time.unwrap_or(5),
            hook: Arc::new(RwLock::new(hooks)),
        }
    }
}

impl<Tz> JobExecutor for DefaultJobExecutor<Tz>
where
    Tz: chrono::TimeZone + Send + Sync + 'static,
    Tz::Offset: Send + Sync,
{
    fn start(&self) -> JoinHandle<()> {
        let storage = self.jobs.to_owned();
        let shutdown_sender = self.shutdown_channel.to_owned();
        let should_next = self.should_next.to_owned();
        let tasks = self.tasks.to_owned();
        let polling_time = self.polling_time;
        let hook = self.hook.to_owned();
        tokio::spawn(async move {
            loop {
                let should_next = *should_next.read().await;

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
                        let handle_action = match hook.read().await.as_deref() {
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
                        let job_context = job.get_job_context().to_owned();
                        let job_execute_result = job.execute().await;
                        let handle_action = match hook.read().await.as_deref() {
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
                    let mut task_vec = tasks.write().await;
                    task_vec.push(handle);
                }

                tokio::time::sleep(std::time::Duration::from_secs(polling_time)).await;
            }
        })
    }

    fn stop(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        let mut shutdown_recv = self.shutdown_channel.subscribe();
        let should_next = self.should_next.to_owned();
        let tasks = self.tasks.to_owned();
        Box::pin(async move {
            *should_next.write().await = false;
            let _ = shutdown_recv.recv().await;
            let tasks = tasks.read().await;
            for i in tasks.iter() {
                loop {
                    if i.is_finished() {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        })
    }
}
