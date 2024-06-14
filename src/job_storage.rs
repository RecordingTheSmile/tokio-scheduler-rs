use async_trait::async_trait;
use chrono::{DateTime, Local};
use cron::Schedule;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;
use tracing::instrument;

use crate::errors::{SchedulerError, SchedulerErrorKind};
use crate::job::{JobContext, ScheduleJob, WillExecuteJobFuture};

///
/// `JobStorage` is used to store all jobs and context.
/// # Usage
/// `register_job`: Register a job
///
/// `add_job`: Add a job with given cron expression and args
///
/// `delete_job`: Delete a exist job
///
/// `has_job`: Judge is there a job with given JobId
///
/// `get_all_should_execute_jobs`: Get all jobs that should execute now
///
/// `restore_jobs`: Restore all jobs from storage
#[async_trait]
pub trait JobStorage<Tz>: Send + Sync
where
    Tz: chrono::TimeZone,
    Tz::Offset: Send + Sync,
{
    ///Register a job
    ///
    /// # Arguments
    /// job: A struct which implements `ScheduleJob` trait.
    ///
    /// # Attention
    /// Every job should provide a unique `job_name` in `get_job_name()` function!
    async fn register_job(&self, job: Box<dyn ScheduleJob>) -> Result<(), SchedulerError>;
    ///Add a job with given cron expression and context.
    ///
    /// # Arguments
    /// 1. `job_name`: A job's name which must be registered before
    ///
    /// 2. `cron`: Cron expression
    ///
    /// 3. `args`: Job args, it's just the alias of `serde_json::Value`
    ///
    /// # Returns
    /// JobId
    async fn add_job(
        &self,
        job_name: &str,
        cron: &str,
        args: &Option<Value>,
    ) -> Result<String, SchedulerError>;
    /// Add a job which is going to retry.
    async fn add_retry_job(
        &self,
        origin_id: &str,
        job_name: &str,
        args: &Option<Value>,
        retry_times: u64,
    ) -> Result<String, SchedulerError>;
    ///Delete a job.
    /// # Arguments
    /// 1. `id`: JobId
    async fn delete_job(&self, id: &str) -> Result<(), SchedulerError>;
    /// Judge is there a job with given JobId
    ///
    /// # Arguments
    /// 1. `id`: JobId
    ///
    /// # Returns
    /// A bool which represents the existence of job with given JobId
    async fn has_job(&self, id: &str) -> Result<bool, SchedulerError>;
    /// Get all jobs which should execute now.
    ///
    /// The return value should be `(JobName,JobId,JobArgs,JobRetryTimes)`
    ///
    /// # Returns
    /// A vec which store all future which should execute now. The last `u64` argument represents `retry_times`. If the task is not in retry queue, this value should be 0.
    async fn get_all_should_execute_jobs(
        &self,
    ) -> Result<Vec<(String, String, Option<Value>, u64)>, SchedulerError>;
    /// Restore all jobs from storage
    async fn restore_jobs(&self) -> Result<(), SchedulerError>;
    /// Get a `job` from `JobStorage` by it's name
    async fn get_job_by_name(
        &self,
        name: &str,
        id: &str,
        args: &Option<Value>,
        retry_times: u64,
    ) -> Result<Option<WillExecuteJobFuture>, SchedulerError>;
    /// Get many `job` from `JobStorage` by it's name
    ///
    /// # Arguments
    /// `exprs` is &Vec<(JobName,JobId,JobArguments)>
    /// # Returns
    /// The return value of this function is `Vec<(WillExecuteJobFuture,JobName,JobId,JobArguments)>`
    ///
    /// **You must return the values in the same order**
    async fn get_jobs_by_name(
        &self,
        exprs: &Vec<(String, String, Option<Value>, u64)>,
    ) -> Result<Vec<(WillExecuteJobFuture, String, String, Option<Value>)>, SchedulerError>;
}

/// Simple Memory Job Storage implements `JobStorage`
///
/// !!! This JobStorage is not recommended for production environment !!!
pub struct MemoryJobStorage<Tz = chrono::Utc>
where
    Tz: chrono::TimeZone + Sync + Send,
{
    tasks: DashMap<String, Box<dyn ScheduleJob>>,
    jobs: DashMap<String, (Schedule, String, Option<Value>)>,
    retry_jobs: DashMap<String, (String, String, Option<Value>, u64)>,
    timezone: Tz,
    last_check_time: Arc<RwLock<DateTime<Tz>>>,
}

impl<Tz> MemoryJobStorage<Tz>
where
    Tz: chrono::TimeZone + Send + Sync,
{
    pub fn new(timezone: Tz) -> Self {
        Self {
            tasks: DashMap::new(),
            jobs: DashMap::new(),
            retry_jobs: DashMap::new(),
            timezone: timezone.to_owned(),
            last_check_time: Arc::new(RwLock::new(Local::now().with_timezone(&timezone))),
        }
    }

    pub fn with_timezone(mut self, tz: Tz) -> Self {
        self.timezone = tz;
        self
    }
}

impl Default for MemoryJobStorage<chrono::Utc> {
    fn default() -> Self {
        Self {
            tasks: DashMap::new(),
            jobs: DashMap::new(),
            retry_jobs: DashMap::new(),
            timezone: chrono::Utc,
            last_check_time: Arc::new(RwLock::new(Local::now().with_timezone(&chrono::Utc))),
        }
    }
}

unsafe impl<Tz: chrono::TimeZone + Sync + Send> Send for MemoryJobStorage<Tz> {}

unsafe impl<Tz: chrono::TimeZone + Sync + Send> Sync for MemoryJobStorage<Tz> {}

#[async_trait]
impl<Tz> JobStorage<Tz> for MemoryJobStorage<Tz>
where
    Tz: chrono::TimeZone + Sync + Send,
    Tz::Offset: Send + Sync,
{
    #[instrument(skip_all)]
    async fn register_job(&self, job: Box<dyn ScheduleJob>) -> Result<(), SchedulerError> {
        let is_registered = self.tasks.get(&job.get_job_name()).is_some();

        if is_registered {
            return Err(SchedulerError::new(SchedulerErrorKind::JobRegistered));
        }

        tracing::debug!("Register job: {}", job.get_job_name());
        self.tasks.insert(job.get_job_name(), job);
        Ok(())
    }

    #[instrument(skip(self))]
    async fn add_job(
        &self,
        job_name: &str,
        cron: &str,
        args: &Option<Value>,
    ) -> Result<String, SchedulerError> {
        let cron = Schedule::from_str(&cron)
            .map_err(|_| SchedulerError::new(SchedulerErrorKind::CronInvalid))?;
        let id = uuid::Uuid::new_v4().to_string();

        self.jobs
            .insert(id.to_owned(), (cron, job_name.to_owned(), args.to_owned()));

        tracing::debug!(
            "Add job: {job_name} with id: {id}",
            job_name = job_name,
            id = id
        );

        Ok(id)
    }

    #[instrument(skip(self))]
    async fn add_retry_job(
        &self,
        origin_id: &str,
        job_name: &str,
        args: &Option<Value>,
        retry_times: u64,
    ) -> Result<String, SchedulerError> {
        tracing::debug!("Add retry job: {job_name}", job_name = job_name);

        let id = uuid::Uuid::new_v4().to_string();

        self.retry_jobs.insert(
            id.to_owned(),
            (
                origin_id.to_owned(),
                job_name.to_owned(),
                args.to_owned(),
                retry_times,
            ),
        );

        Ok(id)
    }

    #[instrument(skip(self))]
    async fn delete_job(&self, id: &str) -> Result<(), SchedulerError> {
        tracing::debug!("Delete job: {id}", id = id);

        self.jobs.remove(id);

        Ok(())
    }

    #[instrument(skip(self))]
    async fn has_job(&self, id: &str) -> Result<bool, SchedulerError> {
        let has = self.jobs.contains_key(id);

        Ok(has)
    }

    #[instrument(skip(self))]
    async fn get_all_should_execute_jobs(
        &self,
    ) -> Result<Vec<(String, String, Option<Value>, u64)>, SchedulerError> {
        let time_now = Local::now().with_timezone(&self.timezone);

        let last_check_at = self.last_check_time.read();
        let cron_and_name: Vec<(Schedule, String, Option<Value>, String)> = self
            .jobs
            .iter()
            .map(|v| {
                (
                    v.value().0.to_owned(),
                    v.value().1.to_owned(),
                    v.value().2.to_owned(),
                    v.key().to_owned(),
                )
            })
            .collect();

        let mut result_vec = vec![];

        for (cron, name, args, id) in cron_and_name {
            for time in cron.after(&last_check_at) {
                if time <= time_now {
                    result_vec.push((name.to_owned(), id.to_owned(), args.to_owned(), 0_u64))
                } else {
                    break;
                }
            }
        }

        drop(last_check_at);
        *self.last_check_time.write() = time_now;

        let mut all_should_retry_jobs = self
            .retry_jobs
            .iter()
            .map(|v| {
                (
                    v.value().1.to_owned(),
                    v.value().0.to_owned(),
                    v.value().2.to_owned(),
                    v.value().3,
                )
            })
            .collect();

        result_vec.append(&mut all_should_retry_jobs);

        self.retry_jobs.clear();

        tracing::debug!("Should execute {} job(s)", result_vec.len());
        Ok(result_vec)
    }

    #[instrument(skip(self))]
    async fn restore_jobs(&self) -> Result<(), SchedulerError> {
        tracing::warn!("MemoryJobStorage does not support restore jobs. Ignored!");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_job_by_name(
        &self,
        name: &str,
        id: &str,
        args: &Option<Value>,
        retry_times: u64,
    ) -> Result<Option<WillExecuteJobFuture>, SchedulerError> {
        if let Some(task) = self.tasks.get(name) {
            tracing::debug!("Found job: {name} with id: {id}", name = name, id = id);
            let job_context = JobContext::new(id.to_owned(), args.to_owned(), retry_times);
            Ok(Some(WillExecuteJobFuture::new(
                task.execute(job_context.to_owned()),
                job_context,
            )))
        } else {
            tracing::warn!("Job: {name} not found", name = name);
            Ok(None)
        }
    }

    #[instrument(skip(self))]
    async fn get_jobs_by_name(
        &self,
        exprs: &Vec<(String, String, Option<Value>, u64)>,
    ) -> Result<Vec<(WillExecuteJobFuture, String, String, Option<Value>)>, SchedulerError> {
        let mut result = vec![];

        for (name, id, args, retry_times) in exprs {
            let job = match self.get_job_by_name(&name, &id, args, *retry_times).await? {
                Some(j) => j,
                None => continue,
            };
            result.push((job, name.to_owned(), id.to_owned(), args.to_owned()));
        }

        tracing::debug!("Found {} job(s)", result.len());
        Ok(result)
    }
}
