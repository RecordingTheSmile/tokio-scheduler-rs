use crate::job::JobReturn;
use serde_json::Value;
use std::future::Future;

/// A trait that defines the hook for job execution
pub trait JobHook: Send + Sync {
    /// Hook that will be called before the job is executed
    ///
    /// # Arguments
    /// * `name` - The name of the job
    /// * `id` - The id of the job
    /// * `args` - The arguments of the job
    /// * `retry_times` - The retry times of the job
    fn before_execute(
        &self,
        name: &str,
        id: &str,
        args: &mut Option<Value>,
        retry_times: u64,
    ) -> impl Future<Output = JobHookReturn> + Send;

    /// Hook that will be called when the job is completed
    ///
    /// # Arguments
    /// * `name` - The name of the job
    /// * `id` - The id of the job
    /// * `args` - The arguments of the job
    /// * `result` - The result of the job
    /// * `retry_times` - The retry times of the job
    fn on_complete(
        &self,
        name: &str,
        id: &str,
        args: &Option<Value>,
        result: &anyhow::Result<JobReturn>,
        retry_times: u64,
    ) -> impl Future<Output = JobHookReturn> + Send;

    /// Hook that will be called when the job is successful
    ///
    /// This hook will be called after `on_complete` hook
    ///
    /// # Arguments
    /// * `name` - The name of the job
    /// * `id` - The id of the job
    /// * `args` - The arguments of the job
    /// * `return_value` - The return value of the job
    /// * `retry_times` - The retry times of the job
    fn on_success(
        &self,
        name: &str,
        id: &str,
        args: &Option<Value>,
        return_value: &JobReturn,
        retry_times: u64,
    ) -> impl Future<Output = JobHookReturn> + Send;

    /// Hook that will be called when the job is failed
    ///
    /// This hook will be called after `on_complete` hook
    ///
    /// # Arguments
    /// * `name` - The name of the job
    /// * `id` - The id of the job
    /// * `args` - The arguments of the job
    /// * `error` - The error of the job
    /// * `retry_times` - The retry times of the job
    fn on_fail(
        &self,
        name: &str,
        id: &str,
        args: &Option<Value>,
        error: &anyhow::Error,
        retry_times: u64,
    ) -> impl Future<Output = JobHookReturn> + Send;
}

/// The return value of the hook
///
/// This value will be used to determine the action after the hook is executed
#[derive(PartialEq, Eq)]
pub enum JobHookReturn {
    /// No action will be taken
    NoAction,
    /// Cancel the job
    ///
    /// This will cancel the job and the job will not be executed
    ///
    /// This will only work in `before_execute` hook
    CancelRunning,
    /// Remove the job
    ///
    /// This will remove the job from the scheduler
    ///
    /// Please note that job will not be unregistered from `JobConsumer`
    RemoveJob,
    /// Reschedule the job
    ///
    /// This will reschedule the job to be executed later
    ///
    /// **NOT** working in `before_execute` hook
    RetryJob,
}
