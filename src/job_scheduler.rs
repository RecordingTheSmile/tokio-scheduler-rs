use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::errors::SchedulerError;
use crate::job::ScheduleJob;
use crate::job_executor::{DefaultJobExecutor, JobExecutor};
use crate::job_storage::{JobStorage, MemoryJobStorage};

/// `JobScheduler` is used to manage and execute jobs and tasks
///
/// # Examples
/// ```rust
/// # use tokio_scheduler_rs::job_scheduler::JobScheduler;
/// let scheduler = JobScheduler::default_with_timezone(chrono_tz::PRC, 30);
/// scheduler.register_job(Box::new(HelloWorldJob)).unwrap();
/// scheduler.add_job("HelloWorldJob".into(),"*/5 * * * * * *".into(),None).await.unwrap();
/// scheduler.restore_jobs().await.unwrap(); // This step is used to restore job execute status.
///                                          // Please notice that you can implement you own job storage to store job status.
/// scheduler.start().await.unwrap(); // `start()` returns a tokio::JoinHandle<()>, you can continue this program if you don't await it.
/// ```
pub struct JobScheduler<'a, Tz: chrono::TimeZone + Send + Sync> {
    job_storage: Arc<dyn JobStorage<Tz> + 'a>,
    job_executor: Arc<dyn JobExecutor + 'a>,
}

impl<'a, Tz> JobScheduler<'a, Tz>
where
    Tz: chrono::TimeZone + Send + Sync + 'static,
    Tz::Offset: Send + Sync,
{
    /// Create a new `JobScheduler` with custom `JobStorage` and `JobExecutor`
    ///
    /// # Example
    /// ```rust
    /// # use std::sync::Arc;
    /// # use tokio_scheduler_rs::job_executor::DefaultJobExecutor;
    /// # use tokio_scheduler_rs::job_scheduler::JobScheduler;
    /// # use tokio_scheduler_rs::job_storage::MemoryJobStorage;
    /// let job_storage = Arc::new(MemoryJobStorage::new(chrono_tz::US));
    /// let scheduler = JobScheduler::new(job_storage,DefaultJobExecutor::new(job_storage));
    /// ```
    pub fn new(
        job_storage: Arc<impl JobStorage<Tz> + 'a>,
        job_executor: impl JobExecutor + 'a,
    ) -> Self {
        Self {
            job_storage,
            job_executor: Arc::new(job_executor),
        }
    }

    /// Create `JobScheduler` with custom Timezone, Default `JobStorage` and Default `JobExecutor`
    ///
    /// # Arguments
    /// `timezone`: The timezone of `JobExecutor`
    /// `stop_timeout`: Set a timeout seconds for tasks to complete  when call the `stop()` fn
    /// equals to
    /// ```rust
    /// # use std::sync::Arc;
    /// # use tokio_scheduler_rs::job_executor::DefaultJobExecutor;
    /// # use tokio_scheduler_rs::job_scheduler::JobScheduler;
    /// # use tokio_scheduler_rs::job_storage::MemoryJobStorage;
    /// let job_storage = Arc::new(MemoryJobStorage::new(chrono::Utc));
    /// let scheduler = JobScheduler::new(job_storage,DefaultJobExecutor::new(job_storage));
    /// ```
    ///
    /// # Example
    /// ```rust
    /// # use tokio_scheduler_rs::job_scheduler::JobScheduler;
    /// let scheduler = JobScheduler::default_with_timezone(chrono_tz::US,30);
    /// ```
    pub fn default_with_timezone(timezone: Tz, stop_timeout: u64) -> Self {
        let storage = Arc::new(MemoryJobStorage::new(timezone));
        Self {
            job_storage: storage.to_owned(),
            job_executor: Arc::new(DefaultJobExecutor::new(storage, None, None, stop_timeout)),
        }
    }

    /// Register a job to `JobStorage`
    ///
    /// # Example
    /// ```rust
    ///
    /// # use tokio_scheduler_rs::job_scheduler::JobScheduler;
    /// let scheduler = JobScheduler::default_with_timezone(chrono::Utc,30);
    /// scheduler.register_job(Box::new(HelloWorldJob)).unwrap();
    /// ```
    pub async fn register_job(&self, job: Box<dyn ScheduleJob>) -> Result<(), SchedulerError> {
        self.job_storage.register_job(job).await?;
        Ok(())
    }

    /// Add a job.
    ///
    /// # Example
    ///```rust
    /// # use tokio_scheduler_rs::job_scheduler::JobScheduler;
    /// let scheduler = JobScheduler::default_with_timezone(chrono::Utc);
    /// scheduler.register_job(Box::new(HelloWorldJob)).unwrap();
    /// let job_id = scheduler.add_job("HelloWorldJob".into(),"*/5 * * * * * *".into(),None).await.unwrap();
    /// ```
    ///
    /// # Returns
    /// This function will return a JobId
    pub async fn add_job(
        &self,
        job_name: &str,
        job_cron: &str,
        args: &Option<serde_json::Value>,
    ) -> Result<String, SchedulerError> {
        self.job_storage.add_job(job_name, job_cron, args).await
    }

    /// Remove a job
    ///
    /// # Arguments
    /// `job_id`: The JobId you got from `add_job()`
    pub async fn delete_job(&self, job_id: &str) -> Result<(), SchedulerError> {
        self.job_storage.delete_job(job_id).await
    }

    /// Judge is there a job with given JobId
    ///
    /// # Arguments
    /// `job_id`: The JobId you got from `add_job()`
    ///
    /// # Returns
    /// A bool which represents the existence of task with given JobId
    pub async fn has_job(&self, job_id: &str) -> Result<bool, SchedulerError> {
        self.job_storage.has_job(job_id).await
    }

    /// Restore all jobs from `JobStorage`.
    ///
    /// Usually used after you restart the program.
    ///
    /// # Notice
    /// You should register all jobs you use before restore jobs.
    ///
    /// # Attention
    /// `JobScheduler` with default `MemoryJobStorage` will do nothing when you execute this function.
    ///
    /// If you want to restore job execution status after restart, you should implement `JobStorage` trait by yourself.
    pub async fn restore_jobs(&self) -> Result<(), SchedulerError> {
        self.job_storage.restore_jobs().await
    }

    /// Start the `JobExecutor` to execute jobs.
    ///
    /// # Returns
    /// It will return a `tokio::JoinHandle<()>`.
    ///
    /// # Notice
    /// If you want to continue running program, you can ignore the `JoinHandle` returned from this function.
    ///
    /// If you want to waiting, you should `.await` it.
    pub fn start(&self) -> JoinHandle<()> {
        let executor = self.job_executor.to_owned();
        executor.start()
    }

    /// Send `stop` signal to `JobExecutor`, and waiting for all jobs to stop.
    ///
    /// # Notice
    /// This function will pending until all running jobs are complete.
    pub fn wait_for_stop(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        self.job_executor.stop()
    }
}
