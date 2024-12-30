mod example_job;

use crate::example_job::ExampleJob;
use chrono::Local;
use chrono_tz::Tz::UTC;
use tokio_scheduler_rs::{
    DefaultJobConsumer, DefaultJobProducer, EmptyHook, JobManager, JobManagerOptions,
};

#[tokio::main]
async fn main() {
    let producer = DefaultJobProducer::new::<chrono_tz::Tz>(UTC);
    let consumer = DefaultJobConsumer::new();

    let mut opts = JobManagerOptions::default();

    opts.graceful_shutdown_timeout_seconds = 10;
    opts.producer_poll_seconds = 1;

    let job_manager = JobManager::new_with_options(producer, consumer, EmptyHook, opts);

    job_manager.register_job(&ExampleJob).await.unwrap();

    job_manager
        .schedule_job(ExampleJob, "* * * * * * *", None)
        .await
        .unwrap();

    println!("Start scheduler");
    job_manager.start().await;

    println!("Current Time: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    println!("Stop scheduler");

    job_manager.stop().await;
    println!("Scheduler stopped");
}
