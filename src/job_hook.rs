use serde_json::Value;
use std::future::Future;
use tokio_scheduler_types::job::JobReturn;
use tokio_scheduler_types::job_hook::{JobHook, JobHookReturn};

pub struct EmptyHook;

impl JobHook for EmptyHook {
    fn before_execute(
        &self,
        _name: &str,
        _id: &str,
        _args: &mut Option<Value>,
        _retry_times: u64,
    ) -> impl Future<Output = JobHookReturn> {
        async move { JobHookReturn::NoAction }
    }

    fn on_complete(
        &self,
        _name: &str,
        _id: &str,
        _args: &Option<Value>,
        _result: &anyhow::Result<JobReturn>,
        _retry_times: u64,
    ) -> impl Future<Output = JobHookReturn> {
        async move { JobHookReturn::NoAction }
    }

    fn on_success(
        &self,
        _name: &str,
        _id: &str,
        _args: &Option<Value>,
        _return_value: &JobReturn,
        _retry_times: u64,
    ) -> impl Future<Output = JobHookReturn> {
        async move { JobHookReturn::NoAction }
    }

    fn on_fail(
        &self,
        _name: &str,
        _id: &str,
        _args: &Option<Value>,
        _error: &anyhow::Error,
        _retry_times: u64,
    ) -> impl Future<Output = JobHookReturn> {
        async move { JobHookReturn::NoAction }
    }
}
