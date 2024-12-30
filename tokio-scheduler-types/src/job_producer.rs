use crate::errors::SchedulerError;
use serde_json::Value;
use std::future::Future;

#[derive(Clone, Debug)]
pub struct JobProduceContext {
    pub job_name: &'static str,
    pub job_id: String,
    pub job_args: Option<Value>,
    pub retry_times: u64,
}

/// A trait that defines the job producer
///
/// The job producer is responsible for producing jobs to be executed
///
/// The backend of a job producer can be a database, a queue, or any other data source
pub trait JobProducer: Send + Sync {
    /// Fetch jobs to be executed
    ///
    /// This function will return a list of `JobProduceContext` which contains the job information
    fn fetch_jobs_to_execute(&self) -> impl Future<Output = Vec<JobProduceContext>> + Send;

    /// Reschedule a retry job
    ///
    /// This function will reschedule a retry job
    ///
    /// # Arguments
    ///
    /// * `ctx` - The job context
    fn reschedule_retry_job(
        &self,
        ctx: JobProduceContext,
    ) -> impl Future<Output = Result<(), SchedulerError>> + Send;

    /// Register a new job with given cron and args
    ///
    /// This function will register a new job with given cron and args
    ///
    /// # Arguments
    ///
    /// * `job_name` - The name of the job
    /// * `execute_at_cron` - The cron expression of the job
    /// * `args` - The arguments of the job
    ///
    /// # Returns
    ///
    /// The id of the job
    fn register_job(
        &self,
        job_name: &'static str,
        execute_at_cron: &str,
        args: Option<Value>,
    ) -> impl Future<Output = Result<String, SchedulerError>>;

    /// Unregister a job
    ///
    /// This function will unregister a job
    ///
    /// # Arguments
    ///
    /// * `job_id` - The id of the job
    fn unregister_job(
        &self,
        job_id: &str,
    ) -> impl Future<Output = Result<Option<&'static str>, SchedulerError>> + Send;

    /// Unregister a job by name
    ///
    /// This function will unregister a job by name
    ///
    /// # Arguments
    ///
    /// * `job_name` - The name of the job
    fn unregister_job_by_name(
        &self,
        job_name: &'static str,
    ) -> impl Future<Output = Result<(), SchedulerError>>;

    /// Check if a job exists
    ///
    /// This function will check if a job exists
    ///
    /// # Arguments
    ///
    /// * `job_id` - The id of the job
    ///
    /// # Returns
    ///
    /// `true` if the job exists, `false` otherwise
    fn has_job(&self, job_id: &str) -> impl Future<Output = Result<bool, SchedulerError>>;

    /// Check if a job is scheduled
    ///
    /// This function will check if a job is scheduled
    ///
    /// # Arguments
    ///
    /// * `job_name` - The name of the job
    ///
    /// # Returns
    ///
    /// `true` if the job is scheduled, `false` otherwise
    fn is_job_scheduled(
        &self,
        job_name: &str,
    ) -> impl Future<Output = Result<bool, SchedulerError>>;
}
