use crate::errors::{SchedulerError, SchedulerErrorKind};
use dashmap::DashMap;
use parking_lot::{Condvar, Mutex};
use serde_json::Value;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_scheduler_types::job::Job;
use tokio_scheduler_types::job_consumer::JobConsumer;
use tokio_scheduler_types::job_hook::{JobHook, JobHookReturn};
use tokio_scheduler_types::job_producer::{JobProduceContext, JobProducer};
use tokio_util::sync::CancellationToken;
use tracing::instrument;

type RunningJobs = DashMap<String, (JoinHandle<()>, CancellationToken)>;

/// `JobManager` is the main struct that manages the job scheduling and execution.
///
/// It is responsible for:
/// - Registering and unregistering jobs
/// - Scheduling jobs
/// - Starting and stopping the job manager
/// - Processing jobs
pub struct JobManager<Producer, Consumer, Hook>
where
    Producer: JobProducer,
    Consumer: JobConsumer,
    Hook: JobHook,
{
    producer: Arc<Producer>,
    consumer: Arc<Consumer>,
    job_hook: Arc<Hook>,
    running_jobs: Arc<RunningJobs>,
    options: JobManagerOptions,
    is_process_exit: Arc<(Mutex<bool>, Condvar)>,
    should_process_exit: Arc<Mutex<bool>>,
}

impl<Producer, Consumer, Hook> JobManager<Producer, Consumer, Hook>
where
    Producer: JobProducer + 'static,
    Consumer: JobConsumer + 'static,
    Hook: JobHook + 'static,
{
    /// Create a new `JobManager` instance.
    pub fn new(producer: Producer, consumer: Consumer, hook: Hook) -> Self {
        Self {
            producer: Arc::new(producer),
            consumer: Arc::new(consumer),
            job_hook: Arc::new(hook),
            running_jobs: Arc::new(RunningJobs::new()),
            options: JobManagerOptions::default(),
            is_process_exit: Arc::new((Mutex::new(false), Condvar::new())),
            should_process_exit: Arc::new(Mutex::new(false)),
        }
    }

    /// Create a new `JobManager` instance with options.
    pub fn new_with_options(
        producer: Producer,
        consumer: Consumer,
        hook: Hook,
        options: JobManagerOptions,
    ) -> Self {
        Self {
            producer: Arc::new(producer),
            consumer: Arc::new(consumer),
            job_hook: Arc::new(hook),
            running_jobs: Arc::new(RunningJobs::new()),
            options,
            is_process_exit: Arc::new((Mutex::new(false), Condvar::new())),
            should_process_exit: Arc::new(Mutex::new(false)),
        }
    }

    /// Register a job to the job manager.
    #[instrument(skip(self, job))]
    pub async fn register_job<T>(&self, job: &'static T) -> Result<(), SchedulerError>
    where
        T: Job + 'static,
    {
        self.consumer.register_job(job).await?;
        Ok(())
    }

    /// Unregister a job from the job manager.
    #[instrument(skip_all)]
    pub async fn unregister_job<T>(&self, job: T) -> Result<(), SchedulerError>
    where
        T: Job,
    {
        self.unregister_job_by_name(job.get_job_name()).await?;
        Ok(())
    }

    /// Auto register all jobs that are marked with the `#[job]` attribute.
    ///
    /// All locations where the `#[job]` attribute is used will be registered as a job.
    #[instrument(skip(self))]
    pub async fn auto_register_job(&self) -> Result<Vec<&'static str>, SchedulerError> {
        self.consumer.auto_register_job().await
    }

    /// Unregister a job by name.
    #[instrument(skip(self))]
    pub async fn unregister_job_by_name(
        &self,
        job_name: &'static str,
    ) -> Result<(), SchedulerError> {
        self.consumer.unregister_job(job_name).await?;
        self.producer.unregister_job_by_name(job_name).await?;
        Ok(())
    }

    /// Schedule a job by name.
    ///
    /// The job will be executed at the specified cron time.
    ///
    /// The `args` parameter is optional and can be used to pass arguments to the job.
    ///
    /// Returns the job id.
    ///
    /// # Errors
    ///
    /// Returns an error if the job does not exist.
    #[instrument(skip(self))]
    pub async fn schedule_job_by_name(
        &self,
        job_name: &'static str,
        execute_at_cron: &str,
        args: Option<Value>,
    ) -> Result<String, SchedulerError> {
        if !self.consumer.has_job(job_name).await? {
            return Err(SchedulerError::new(SchedulerErrorKind::JobNotExists));
        }
        self.producer
            .register_job(job_name, execute_at_cron, args)
            .await
    }

    /// Schedule a job.
    ///
    /// The job will be executed at the specified cron time.
    ///
    /// The `args` parameter is optional and can be used to pass arguments to the job.
    ///
    /// Returns the job id.
    ///
    /// # Errors
    ///
    /// Returns an error if the job does not exist.
    #[instrument(skip(self, job))]
    pub async fn schedule_job<T>(
        &self,
        job: T,
        execute_at_cron: &str,
        args: Option<Value>,
    ) -> Result<String, SchedulerError>
    where
        T: Job,
    {
        let job_name = job.get_job_name();
        self.schedule_job_by_name(job_name, execute_at_cron, args)
            .await
    }

    /// Judge whether a job is scheduled by job id.
    ///
    /// Returns `true` if the job is scheduled, otherwise `false`.
    #[instrument(skip(self))]
    pub async fn has_job(&self, job_id: &str) -> Result<bool, SchedulerError> {
        self.producer.has_job(job_id).await
    }

    /// Judge whether a job is registered by given job name.
    ///
    /// Returns `true` if the job is registered, otherwise `false`.
    #[instrument(skip_all)]
    pub async fn is_job_registered_by_name(&self, job_name: &str) -> Result<bool, SchedulerError> {
        self.consumer.has_job(job_name).await
    }

    /// Judge whether a job is registered by job.
    ///
    /// Returns `true` if the job is registered, otherwise `false`.
    #[instrument(skip_all)]
    pub async fn is_job_registered<T>(&self, job: T) -> Result<bool, SchedulerError>
    where
        T: Job,
    {
        self.is_job_registered_by_name(job.get_job_name()).await
    }

    /// Judge whether a job is scheduled by given job name.
    ///
    /// Returns `true` if the job is scheduled, otherwise `false`.
    #[instrument(skip(self))]
    pub async fn is_job_scheduled_by_name(&self, job_name: &str) -> Result<bool, SchedulerError> {
        self.producer.is_job_scheduled(job_name).await
    }

    /// Judge whether a job is scheduled by job.
    ///
    /// Returns `true` if the job is scheduled, otherwise `false`.
    #[instrument(skip_all)]
    pub async fn is_job_scheduled<T>(&self, job: T) -> Result<bool, SchedulerError>
    where
        T: Job,
    {
        self.is_job_scheduled_by_name(job.get_job_name()).await
    }

    /// Process each job by given context.
    #[instrument(skip_all)]
    async fn process_job(
        hook: Arc<Hook>,
        producer: Arc<Producer>,
        consumer: Arc<Consumer>,
        mut job_context: JobProduceContext,
        cancellation_token: CancellationToken,
    ) {
        let handle_remove_job = || async {
            let mut remove_success = true;

            let producer_result = producer.unregister_job(&job_context.job_id).await;

            if let Err(e) = producer_result {
                tracing::error!("Failed to remove job from producer: {:?}", e);
                remove_success = false;
            }

            remove_success
        };

        let hook_result = hook
            .before_execute(
                job_context.job_name,
                &job_context.job_id,
                &mut job_context.job_args,
                job_context.retry_times,
            )
            .await;

        match hook_result {
            JobHookReturn::CancelRunning => {
                return;
            }
            JobHookReturn::RemoveJob => {
                handle_remove_job().await;
                return;
            }
            _ => {}
        }

        if cancellation_token.is_cancelled() {
            return;
        }

        let job_result_return = consumer
            .consume(job_context.to_owned(), cancellation_token.to_owned())
            .await;

        if cancellation_token.is_cancelled() {
            return;
        }

        let hook_result = hook
            .on_complete(
                job_context.job_name,
                &job_context.job_id,
                &job_context.job_args,
                &job_result_return,
                job_context.retry_times,
            )
            .await;

        let mut job_removed = false;
        let mut job_rescheduled = false;

        match hook_result {
            JobHookReturn::RemoveJob => {
                job_removed = handle_remove_job().await;
            }
            JobHookReturn::RetryJob => {
                if let Err(e) = producer.reschedule_retry_job(job_context.to_owned()).await {
                    tracing::error!(
                        "Failed to reschedule retry job when executing on_complete hook: {:?}",
                        e
                    );
                } else {
                    job_rescheduled = true;
                }
            }
            _ => {}
        }

        if cancellation_token.is_cancelled() {
            return;
        }

        let hook_result = match job_result_return {
            Ok(return_value) => {
                hook.on_success(
                    job_context.job_name,
                    &job_context.job_id,
                    &job_context.job_args,
                    &return_value,
                    job_context.retry_times,
                )
                .await
            }
            Err(error) => {
                hook.on_fail(
                    job_context.job_name,
                    &job_context.job_id,
                    &job_context.job_args,
                    &error,
                    job_context.retry_times,
                )
                .await
            }
        };

        if cancellation_token.is_cancelled() {
            return;
        }

        match hook_result {
            JobHookReturn::RemoveJob => {
                if !job_removed {
                    handle_remove_job().await;
                }
            }
            JobHookReturn::RetryJob => {
                if !job_rescheduled {
                    if let Err(e) = producer.reschedule_retry_job(job_context.to_owned()).await {
                        tracing::error!("Failed to reschedule retry job when executing on_success/on_fail hook: {:?}", e);
                    }
                }
            }
            _ => {}
        }
    }

    /// Poll jobs to execute.
    #[instrument(skip_all)]
    async fn poll_jobs(
        running_jobs: Arc<RunningJobs>,
        producer: Arc<Producer>,
        consumer: Arc<Consumer>,
        job_hook: Arc<Hook>,
        should_exit: Arc<Mutex<bool>>,
        exit_cond_pair: Arc<(Mutex<bool>, Condvar)>,
        options: JobManagerOptions,
    ) {
        loop {
            if *should_exit.lock() {
                let (lock, cvar) = &*exit_cond_pair;
                let mut exit = lock.lock();
                *exit = true;
                cvar.notify_all();
                return;
            }

            // Remove all completed jobs from running_jobs
            running_jobs.retain(|id, (handle, _)| {
                if handle.is_finished() {
                    tracing::debug!("Job with job_id {} is completed", id);
                    false
                } else {
                    true
                }
            });

            let job_ctxs = producer.fetch_jobs_to_execute().await;

            for job_ctx in job_ctxs {
                let producer = producer.to_owned();
                let consumer = consumer.to_owned();
                let job_context = job_ctx.clone();
                let hook = job_hook.to_owned();
                let cancellation_token = CancellationToken::new();
                let job_handle = tokio::spawn(Self::process_job(
                    hook,
                    producer,
                    consumer,
                    job_context,
                    cancellation_token.to_owned(),
                ));
                running_jobs.insert(job_ctx.job_id.clone(), (job_handle, cancellation_token));
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(
                options.producer_poll_seconds,
            ))
            .await;
        }
    }

    /// Start the job manager.
    ///
    /// This will start the job manager and begin processing jobs.
    ///
    /// Returns a `JoinHandle` that can be used to await the completion of the job manager.
    #[instrument(skip_all)]
    pub async fn start(&self) -> JoinHandle<()> {
        let running_jobs = self.running_jobs.clone();
        let producer = self.producer.clone();
        let consumer = self.consumer.clone();
        let hook = self.job_hook.clone();
        let options = self.options;
        let should_exit = self.should_process_exit.clone();
        let exit_cond_pair = self.is_process_exit.clone();
        tokio::spawn(async move {
            Self::poll_jobs(
                running_jobs,
                producer,
                consumer,
                hook,
                should_exit,
                exit_cond_pair,
                options,
            )
            .await;
        })
    }

    /// Stop the job manager.
    ///
    /// This will stop the job manager and cancel all running jobs.
    #[instrument(skip_all)]
    pub async fn stop(&self) {
        *self.should_process_exit.lock() = true;
        let (lock, cvar) = &*self.is_process_exit;
        let mut exit = lock.lock();
        while !*exit {
            cvar.wait(&mut exit);
        }

        let running_job_keys = self
            .running_jobs
            .iter()
            .map(|k| k.key().to_owned())
            .collect::<Vec<_>>();

        let mut wait_queue = vec![];

        for job_key in running_job_keys {
            if let Some((_, (handle, abort_signal))) = self.running_jobs.remove(&job_key) {
                abort_signal.cancel();
                wait_queue.push((job_key, handle));
            }
        }

        while let Some((job_key, handle)) = wait_queue.pop() {
            let abort_handler = handle.abort_handle();
            tokio::select! {
                _ = handle => {}
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(self.options.graceful_shutdown_timeout_seconds)) => {
                    abort_handler.abort();
                    tracing::warn!("Job with job_id {job_key} is not finished after graceful shutdown timeout, terminating forcefully");
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct JobManagerOptions {
    pub producer_poll_seconds: u64,
    pub graceful_shutdown_timeout_seconds: u64,
}

impl Default for JobManagerOptions {
    fn default() -> Self {
        Self {
            producer_poll_seconds: 5,
            graceful_shutdown_timeout_seconds: 30,
        }
    }
}
