use std::sync::Arc;

use example_jobs::ExampleJob;
use tokio_scheduler_rs::{DefaultJobExecutor, JobScheduler, MemoryJobStorage};

pub mod example_jobs;

#[tokio::main]
async fn main() {
    // Create a new `job_storage`, you can impl it by yourself.
    // !!!  PLEASE NOTICE THAT MEMORYJOBSTORAGE SHOULD NOT BE USED IN PRODUCTION  !!!
    let job_storage = Arc::new(MemoryJobStorage::new(chrono::Utc));
    // Create a new `job_executor`.
    // The second `u64` args means how long does `DefaultJobExecutor` query the `job_storage`.
    // If you set it to `None`, Some(5) is used as default.
    let job_executor = DefaultJobExecutor::new(job_storage.to_owned(), Some(1), None, 30);
    let scheduler = JobScheduler::new(job_storage, job_executor);

    // Register a job
    scheduler.register_job(Box::new(ExampleJob)).await.unwrap();

    // Set a schedule with given cron expression.
    // !!! PLEASE NOTICE THAT YOU MUST REGISTER THE JOB FIRST !!!
    scheduler
        .add_job(ExampleJob::JOB_NAME, "*/5 * * * * * *", &None)
        .await
        .unwrap();

    // Don't forget to start it.
    println!("Start scheduler!");
    scheduler.start();

    tokio::time::sleep(std::time::Duration::from_secs(15)).await;

    // Wait for all jobs are processed and stop the schedule.
    // The `JobExecutor` will stop execute NEW job once you execute this.
    println!("Stop scheduler!");
    scheduler.wait_for_stop().await;
}
