use tokio_scheduler_rs::{
    async_trait,
    job_hook::{JobHook, JobHookReturn},
    Value,
};

pub struct ExampleHook;

#[async_trait]
impl JobHook for ExampleHook {
    async fn on_execute(&self, name: &str, id: &str, args: &Option<Value>) -> JobHookReturn {
        println!(
            "Task: {} with id: {} and args: {:#?} is going to execute!",
            name, id, args
        );
        JobHookReturn::NoAction
        // If you want to Cancel this running ONLY THIS TIME:
        // JobHookReturn::CancelRunning
        // or you want to Cancel this running and remove this schedule forever:
        // JobHookReturn::RemoveJob
    }
    async fn on_complete(
        &self,
        name: &str,
        id: &str,
        args: &Option<Value>,
        result: &Result<Value, Value>,
        retry_times: u64,
    ) -> JobHookReturn {
        println!(
            "Task: {} with id: {} and args: {:#?} is complete! Result is: {:#?}, retry time is: {}",
            name, id, args, result, retry_times
        );
        JobHookReturn::NoAction
        // If you want to Cancel this running and remove this schedule forever:
        // JobHookReturn::RemoveJob
        // Or if you want to retry this job:
        // JobHookReturn::RetryJob
    }
    async fn on_success(
        &self,
        name: &str,
        id: &str,
        args: &Option<Value>,
        return_vaule: &Value,
        retry_times: u64,
    ) -> JobHookReturn {
        println!(
            "Task: {} with id: {} and args: {:#?} is complete! ReturnValue is: {:#?}, retry time is: {}",
            name, id, args, return_vaule, retry_times
        );
        JobHookReturn::NoAction
        // If you want to Cancel this running and remove this schedule forever:
        // JobHookReturn::RemoveJob
        // Or if you want to retry this job:
        // JobHookReturn::RetryJob
    }
    async fn on_fail(
        &self,
        name: &str,
        id: &str,
        args: &Option<Value>,
        error: &Value,
        retry_times: u64,
    ) -> JobHookReturn {
        println!(
            "Task: {} with id: {} and args: {:#?} is complete! Error is: {:#?}, retry time is: {}",
            name, id, args, error, retry_times
        );
        JobHookReturn::NoAction
        // If you want to Cancel this running and remove this schedule forever:
        // JobHookReturn::RemoveJob
        // Or if you want to retry this job:
        // JobHookReturn::RetryJob
    }
}
