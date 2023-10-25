/*!
<div style="text-align: center"><h1>TOKIO-SCHEDULER-RS</h1></div>

<div style="text-align: center">Yet Another JobScheduler</div>

<div style="text-align: center"><a href="README_CN.MD">简体中文</a></div>

# Features
* Async Completely
* Witten with tokio runtime
* Maximum Customize
* Hook support
* Automatic retry support
* Distribute Job Execution support (You should implement it by yourself)

# Example
```rust
use std::sync::Arc;

use tokio_scheduler_rs::{job_hook::JobHook,job_hook::JobHookReturn,async_trait,DefaultJobExecutor, JobScheduler, MemoryJobStorage, JobContext, JobFuture,Value,ScheduleJob};

struct ExampleJob;

impl ScheduleJob for ExampleJob{
fn get_job_name(&self) -> String {
        String::from("ExampleJob")
    }

fn execute(&self, ctx: JobContext) -> JobFuture {
        Box::pin(async move{
            println!("Hello, World! My JobId is {}",ctx.get_id());
            Ok(Value::default())
        })
    }
}

struct ExampleHook;

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
        result: &anyhow::Result<Value>,
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
        error: &anyhow::Error,
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

#[tokio::main]
async fn main() {
    // Create a new `job_storage`, you can impl it by yourself.
    // !!!  PLEASE NOTICE THAT MEMORYJOBSTORAGE SHOULD NOT BE USED IN PRODUCTION  !!!
    let job_storage = Arc::new(MemoryJobStorage::new(chrono::Utc));
    // Create a new `job_executor`.
    // You should register your job hook here
    let job_executor = DefaultJobExecutor::new(
        job_storage.to_owned(),
        Some(10),
        Some(Box::new(ExampleHook)),
        30
    );
    let scheduler = JobScheduler::new(job_storage, job_executor);

    // Register a job
    scheduler.register_job(Box::new(ExampleJob)).await.unwrap();

    // Set a schedule with given cron expression.
    // !!! PLEASE NOTICE THAT YOU MUST REGISTER THE JOB FIRST !!!
    scheduler
        .add_job(&ExampleJob.get_job_name(), "* * * * * * *", &None)
        .await
        .unwrap();

    // Don't forget to start it.
    println!("Start scheduler");
    scheduler.start();

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    // Wait for all jobs are processed and stop the schedule.
    // The `JobExecutor` will stop execute NEW job once you execute this.
    println!("Stop scheduler");
    scheduler.wait_for_stop().await;
}
```
# Examples
You can see examples in the `examples` directory.

# Contribute
If you have some ideas, you can create a pull request or open an issue.

Any kinds of contributions are welcome!

# Distributed Job Execution
As you can see, in `JobExecutor` trait, we define two functions: `start` and `stop`.

So, you can define your own `JobExecutor`, polling the `JobStorage` and transfer job information to remote via anyway you want.

In the remote machine, you should define the `Job` with given `name`, then execute it with given job information(`new JobContext()`) received from origin `JobExecutor` machine.

![DistributedJob](assets/DistributedJob.jpg "Distributed Job Execution")

# Roadmap
* Add more tests
* Distributed task execution system that is ready-to-use out of the box

# License
MIT
 */
pub mod errors;
pub mod job;
pub mod job_executor;
pub mod job_hook;
pub mod job_scheduler;
pub mod job_storage;
pub use anyhow::anyhow as create_error;
pub use anyhow::bail;
pub use anyhow::Error;
pub use anyhow::Result;
pub use async_trait::async_trait;
pub use job::JobContext;
pub use job::JobFuture;
pub use job::ScheduleJob;
pub use job::WillExecuteJobFuture;
pub use job_executor::DefaultJobExecutor;
pub use job_executor::JobExecutor;
pub use job_scheduler::JobScheduler;
pub use job_storage::JobStorage;
pub use job_storage::MemoryJobStorage;
pub use serde_json;
pub use serde_json::Value;
