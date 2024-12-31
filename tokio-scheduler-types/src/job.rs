use serde::Serialize;
use serde_json::Value;
use std::fmt::Display;
use std::future::Future;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

/// All jobs should implements this trait
///
/// # Example
/// ```rust
/// # use std::future::Future;
/// # use std::pin::Pin;
/// # use async_trait::async_trait;
/// # use tokio_scheduler_types::job::{Job, JobContext, JobFuture, JobReturn};/// #
///
/// pub struct TestJob;
///
///     impl Job for TestJob{
///         fn get_job_name(&self) -> &'static str {
///             "TestJob"
///         }
///
///         fn execute(&self,ctx: JobContext) -> JobFuture {
///             Box::pin(async move{
///                 println!("Hello,World! My Task Uuid is: {}",ctx.get_id());
///                 Ok(JobReturn::default())
///             })
///         }
///    }
/// ```
/// # Attention
/// `job_name` must be unique.
pub trait Job: Send + Sync {
    fn get_job_name(&self) -> &'static str;
    fn execute(&self, ctx: JobContext) -> JobFuture;
}

/// A future that will return a `JobReturn` when resolved
pub type JobFuture = Pin<Box<dyn Future<Output = anyhow::Result<JobReturn>> + Send + Sync>>;

/// A struct that stores the return value of a job
///
/// # Example
///
/// ```rust
/// # use serde::Serialize;
/// # use serde_json::Value;
/// # use tokio_scheduler_types::job::JobReturn;
///
/// let mut job_return = JobReturn::default();
/// job_return.set("hello", "world");
/// job_return.set_typed("hello", "world").unwrap();
/// assert_eq!(job_return.get("hello").unwrap().as_str().unwrap(), "world");
/// assert_eq!(job_return.get_typed::<String>("hello").unwrap(), "world");
///
/// ```
#[derive(Clone, Debug)]
pub struct JobReturn {
    inner: Value,
}

impl JobReturn {
    /// Create a new `JobReturn` with given value
    pub fn new(value: Value) -> Self {
        Self { inner: value }
    }

    /// Get the inner value of `JobReturn`
    pub fn get_value(&self) -> &Value {
        &self.inner
    }

    /// Consume `JobReturn` and return the inner value
    pub fn into_value(self) -> Value {
        self.inner
    }

    /// Set a key-value pair to `JobReturn`
    pub fn set(&mut self, key: &str, value: impl Into<Value>) {
        self.inner[key] = value.into();
    }

    /// Set a key-value pair to `JobReturn` with typed value
    pub fn set_typed(&mut self, key: &str, value: impl Serialize) -> Result<(), serde_json::Error> {
        self.inner[key] = serde_json::to_value(value)?;
        Ok(())
    }

    /// Get a value reference from `JobReturn`
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.inner.get(key)
    }

    /// Get a typed value from `JobReturn`
    pub fn get_typed<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.inner.get(key).and_then(|v| T::deserialize(v).ok())
    }
}

impl Default for JobReturn {
    /// Create a new `JobReturn` with default (empty) value
    fn default() -> Self {
        Self {
            inner: Value::default(),
        }
    }
}

impl From<Value> for JobReturn {
    fn from(value: Value) -> Self {
        Self { inner: value }
    }
}

impl From<JobReturn> for Value {
    fn from(job_return: JobReturn) -> Self {
        job_return.inner
    }
}

impl Display for JobReturn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.to_string())
    }
}

/// A context which stores job information when running
#[derive(Clone, Debug)]
pub struct JobContext {
    args: Option<Value>,
    retry_times: u64,
    id: String,
    cancellation_token: CancellationToken,
}

impl JobContext {
    ///
    /// Create a new `JobContext` with given args
    pub fn new(
        id: String,
        args: Option<Value>,
        retry_times: u64,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            id,
            args,
            retry_times,
            cancellation_token,
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

    /// Set a key-value pair to `JobContext`
    pub fn set_arg(&mut self, key: &str, value: impl Into<Value>) {
        let mut args = self.args.take().unwrap_or_default();
        args[key] = value.into();
        self.args = Some(args);
    }

    /// Set a key-value pair to `JobContext` with typed value
    pub fn set_typed_arg(
        &mut self,
        key: &str,
        value: impl Serialize,
    ) -> Result<(), serde_json::Error> {
        let mut args = self.args.take().unwrap_or_default();
        args[key] = serde_json::to_value(value)?;
        self.args = Some(args);
        Ok(())
    }

    /// Get a value reference from `JobContext`
    pub fn get_arg(&self, key: &str) -> Option<&Value> {
        self.args.as_ref().and_then(|args| args.get(key))
    }

    /// Get a typed value from `JobContext`
    pub fn get_typed_arg<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.args
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(|v| T::deserialize(v).ok())
    }

    /// Get the job's cancellation token
    pub fn get_cancellation_token(&self) -> &CancellationToken {
        &self.cancellation_token
    }

    /// Cancel the job
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    /// Check if the job is cancelled
    ///
    /// # Returns
    ///
    /// `true` if the job is cancelled, `false` otherwise
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    /// Wait for the job to be cancelled
    ///
    /// # Returns
    ///
    /// A future that will resolve when the job is cancelled
    pub fn wait_for_cancel(&self) -> impl Future<Output = ()> + use<'_> {
        self.cancellation_token.cancelled()
    }
}

impl Default for JobContext {
    /// Create a new `JobContext` with default value
    fn default() -> Self {
        Self {
            args: None,
            retry_times: 0,
            id: String::new(),
            cancellation_token: CancellationToken::new(),
        }
    }
}

inventory::collect!(&'static dyn Job);

#[cfg(test)]
mod test_job {
    use crate::job::{Job, JobContext, JobFuture, JobReturn};
    use serde::{Deserialize, Serialize};
    use tokio_util::sync::CancellationToken;

    pub struct TestJob;

    #[derive(Serialize, Deserialize)]
    struct TestReturnValue {
        hello: String,
    }
    impl Job for TestJob {
        fn get_job_name(&self) -> &'static str {
            "TestJob"
        }

        fn execute(&self, ctx: JobContext) -> JobFuture {
            Box::pin(async move {
                println!("Hello,World! My Task Uuid is: {}", ctx.get_id());
                assert_eq!(ctx.get_retry_times(), 0);
                assert_eq!(ctx.get_arg("hello").unwrap().as_str().unwrap(), "world");
                assert_eq!(ctx.get_id(), "test");
                let mut ret = JobReturn::default();
                ret.set_typed(
                    "hello",
                    TestReturnValue {
                        hello: "world".to_string(),
                    },
                )?;
                Ok(ret)
            })
        }
    }

    async fn handle_dyn_job(job: &dyn Job) {
        let mut ctx = JobContext::new("test".to_string(), None, 0, CancellationToken::new());

        ctx.set_arg("hello", "world");

        let result = job.execute(ctx).await;
        assert!(result.is_ok());
        assert_eq!(
            result
                .unwrap()
                .get_typed::<TestReturnValue>("hello")
                .unwrap()
                .hello,
            "world"
        );
    }

    #[tokio::test]
    async fn test_job() {
        handle_dyn_job(&TestJob).await;
    }
}
