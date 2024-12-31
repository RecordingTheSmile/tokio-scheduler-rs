use crate::errors::SchedulerError;
use crate::job::{Job, JobReturn};
use crate::job_producer::JobProduceContext;
use std::future::Future;
use tokio_util::sync::CancellationToken;

/// Job consumer trait
///
/// This trait is used to consume jobs that are produced by the job producer.
pub trait JobConsumer: Send + Sync {
    /// Consume a job
    ///
    /// # Arguments
    /// * `ctx` - The job context
    /// * `cancellation_token` - The cancellation token
    ///
    /// # Returns
    ///
    /// A future that will resolve to the job return value
    fn consume(
        &self,
        ctx: JobProduceContext,
        cancellation_token: CancellationToken,
    ) -> impl Future<Output = anyhow::Result<JobReturn>> + Send + Sync;

    /// Register a job
    ///
    /// # Arguments
    /// * `job` - The job to register
    ///
    /// # Returns
    ///
    /// A future that will resolve to `Ok(())` if the job is registered successfully
    fn register_job(
        &self,
        job: &'static dyn Job,
    ) -> impl Future<Output = Result<(), SchedulerError>>;

    /// Unregister a job
    ///
    /// # Arguments
    /// * `job_name` - The name of the job to unregister
    ///
    /// # Returns
    ///
    /// A future that will resolve to `Ok(())` if the job is unregistered successfully
    fn unregister_job(
        &self,
        job_name: &str,
    ) -> impl Future<Output = Result<(), SchedulerError>> + Send;

    /// Auto register jobs
    ///
    /// # Returns
    ///
    /// A future that will resolve to a vector of job names that were registered
    fn auto_register_job(&self) -> impl Future<Output = Result<Vec<&'static str>, SchedulerError>> {
        async move {
            let jobs_mark_by_macro = inventory::iter::<&'static dyn Job>();

            let mut ret = vec![];
            for job in jobs_mark_by_macro {
                self.register_job(job.to_owned()).await?;
                ret.push(job.get_job_name());
            }

            Ok(ret)
        }
    }

    /// Check if a job exists
    ///
    /// # Arguments
    /// * `job_name` - The name of the job to check
    ///
    /// # Returns
    ///
    /// A future that will resolve to `true` if the job exists
    fn has_job(&self, job_name: &str) -> impl Future<Output = Result<bool, SchedulerError>>;
}
