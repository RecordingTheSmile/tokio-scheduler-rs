use std::future::Future;
use std::pin::Pin;

use serde_json::Value;

/// All jobs should implements this trait
///
/// # Example
/// ```rust
/// # use std::future::Future;
/// # use std::pin::Pin;
/// # use tokio_scheduler_rs::job::ScheduleJob;
/// use tokio_scheduler_rs::JobContext;
/// pub struct TestJob;
///
///     impl ScheduleJob for TestJob{
///         fn get_job_name(&self) -> String {
///             String::from("TestJob")
///         }
///
///         fn execute(&self,ctx: &mut JobContext) -> Pin<Box<dyn Future<Output=()>>> {
///             Box::pin(async move{
///                 println!("Hello,World! My Task Uuid is: {}",ctx.get_id());
///             })
///         }
///    }
/// ```
/// # Attention
/// `job_name` must be unique!!!
pub trait ScheduleJob: Send + Sync {
    fn get_job_name(&self) -> String;
    fn execute(&self, ctx: &mut JobContext) -> JobFuture;
}

/// A context which stores job information when running
#[derive(Clone, Debug)]
pub struct JobContext {
    args: Option<Value>,
    retry_times: u64,
    id: String,
    should_be_deleted: bool,
    should_retry: bool,
}

impl JobContext {
    ///
    /// Create a new `JobContext` with given args
    pub fn new(id: String, args: Option<Value>, retry_times: u64) -> Self {
        Self {
            id,
            args,
            retry_times,
            should_be_deleted: false,
            should_retry: false,
        }
    }

    /// Set `JobContext`'s args
    pub fn set_args(&mut self, args: Option<Value>) {
        self.args = args;
    }

    /// Set `JobContext`'s retry_times
    pub fn set_retry_times(&mut self, retry_times: u64) {
        self.retry_times = retry_times;
    }

    /// Set `JobContext`'s id
    pub fn set_id(&mut self, id: &str) {
        self.id = id.to_owned();
    }

    /// Set `JobContext`'s retry_times +1
    pub fn add_retry_times(&mut self) {
        self.retry_times += 1;
    }

    /// Get total retry times for this job
    pub fn get_retry_times(&self) -> u64 {
        self.retry_times
    }

    /// Get a cloned args for this job. If args is `None`, `Value::default()` will be returned.
    pub fn get_args(&self) -> Value {
        self.args.to_owned().unwrap_or_default()
    }

    /// Get a args reference.
    pub fn get_option_args(&self) -> Option<&Value> {
        self.args.as_ref()
    }

    /// Get the job's id
    pub fn get_id(&self) -> &str {
        self.id.as_str()
    }

    /// Mark this job as delete. **This can be override by job hook.**
    pub fn delete(&mut self) {
        self.should_be_deleted = true;
    }

    /// Mark this job as retry. **This can be override by job hook.**
    pub fn retry(&mut self) {
        self.should_retry = true;
    }

    /// Get if this job should be deleted.
    pub fn is_delete_scheduled(&self) -> bool {
        self.should_be_deleted
    }

    /// Get if this job should be retried.
    pub fn is_retry_scheduled(&self) -> bool {
        self.should_retry
    }
}

impl Default for JobContext {
    fn default() -> Self {
        Self {
            args: None,
            retry_times: 0,
            id: String::new(),
            should_retry: false,
            should_be_deleted: false,
        }
    }
}

pub type JobFuture = Pin<Box<dyn Future<Output = anyhow::Result<Value>> + Send>>;

pub struct WillExecuteJobFuture {
    job_future: JobFuture,
    context: JobContext,
}

impl WillExecuteJobFuture {
    pub fn new(job_future: JobFuture, job_context: JobContext) -> Self {
        Self {
            job_future,
            context: job_context,
        }
    }

    pub async fn execute(self) -> (JobContext, anyhow::Result<Value>) {
        (self.context, self.job_future.await)
    }

    pub fn get_job_context(&self) -> &JobContext {
        &self.context
    }
}
