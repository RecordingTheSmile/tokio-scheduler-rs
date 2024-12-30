use serde_json::Value;
use std::future::Future;
use tokio_scheduler_rs::JobHook;
use tokio_scheduler_types::job::JobReturn;
use tokio_scheduler_types::job_hook::JobHookReturn;

pub(crate) struct ExampleHook;

impl JobHook for ExampleHook {
    fn before_execute(
        &self,
        _name: &str,
        _id: &str,
        _args: &mut Option<Value>,
        _retry_times: u64,
    ) -> impl Future<Output = JobHookReturn> {
        async move {
            println!("Before execute hook");
            JobHookReturn::NoAction
        }
    }

    fn on_complete(
        &self,
        _name: &str,
        _id: &str,
        _args: &Option<Value>,
        _result: &anyhow::Result<JobReturn>,
        _retry_times: u64,
    ) -> impl Future<Output = JobHookReturn> {
        async move {
            println!("On complete hook");
            JobHookReturn::NoAction
        }
    }

    fn on_success(
        &self,
        _name: &str,
        _id: &str,
        _args: &Option<Value>,
        _return_value: &JobReturn,
        _retry_times: u64,
    ) -> impl Future<Output = JobHookReturn> {
        async move {
            println!("On success hook");
            JobHookReturn::NoAction
        }
    }

    fn on_fail(
        &self,
        _name: &str,
        _id: &str,
        _args: &Option<Value>,
        _error: &anyhow::Error,
        _retry_times: u64,
    ) -> impl Future<Output = JobHookReturn> {
        async move {
            println!("On fail hook");
            JobHookReturn::NoAction
        }
    }
}
