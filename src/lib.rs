/// <div style="text-align: center"><h1>TOKIO-SCHEDULER-RS</h1></div>
///
/// <div style="text-align: center">Yet Another JobScheduler</div>
///
/// <a href="README_CN.MD">简体中文</a>
///
/// # Features
/// * Async Completely
/// * Witten with tokio runtime
/// * Maximum Customize
/// * Hook support
/// * Automatic retry support
/// * Distribute Job Execution support (You should implement it by yourself)
///
/// # Example
/// ```rust
/// use std::sync::Arc;
///
/// use example_hook::ExampleHook;
/// use example_jobs::ExampleJob;
/// use tokio_scheduler_rs::{DefaultJobExecutor, JobScheduler, MemoryJobStorage};
///
/// #[tokio::main]
/// async fn main() {
///     // Create a new `job_storage`, you can impl it by yourself.
///     // !!!  PLEASE NOTICE THAT MEMORYJOBSTORAGE SHOULD NOT BE USED IN PRODUCTION  !!!
///     let job_storage = Arc::new(MemoryJobStorage::new(chrono::Utc));
///     // Create a new `job_executor`.
///     // You should register your job hook here
///     let job_executor = DefaultJobExecutor::new(
///         job_storage.to_owned(),
///         Some(10),
///         Some(Box::new(ExampleHook)),
///         30
///     );
///     let scheduler = JobScheduler::new(job_storage, job_executor);
///
///     // Register a job
///     scheduler.register_job(Box::new(ExampleJob)).await.unwrap();
///
///     // Set a schedule with given cron expression.
///     // !!! PLEASE NOTICE THAT YOU MUST REGISTER THE JOB FIRST !!!
///     scheduler
///         .add_job(ExampleJob::JOB_NAME, "*/5 * * * * * *", &None)
///         .await
///         .unwrap();
///
///     // Don't forget to start it.
///     scheduler.start().await.unwrap();
///     // If you don't want to wait it, then:
///     // scheduler.start();
///
///     tokio::time::sleep(std::time::Duration::from_secs(60)).await;
///
///     // Wait for all jobs are processed and stop the schedule.
///     // The `JobExecutor` will stop execute NEW job once you execute this.
///     scheduler.wait_for_stop().await;
/// }
/// ```
/// # Examples
/// You can see examples in the `examples` directory.
///
/// # Contribute
/// If you have some ideas, you can create a pull request or open an issue.
///
/// Any kinds of contributions are welcome!
///
/// # Distributed Job Execution
/// As you can see, in `JobExecutor` trait, we define two functions: `start` and `stop`.
///
/// So, you can define your own `JobExecutor`, polling the `JobStorage` and transfer job information to remote via anyway you want.
///
/// In the remote machine, you should define the `Job` with given `name`, then execute it with given job information(`new JobContext()`) received from origin `JobExecutor` machine.
///
/// ![DistributedJob](assets/DistributedJob.jpg "Distributed Job Execution")
///
/// # Roadmap
/// * Add more tests
/// * Distributed task execution system that is ready-to-use out of the box
///
/// # License
/// MIT
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
