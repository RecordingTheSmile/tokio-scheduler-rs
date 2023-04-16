//! # TOKIO-SCHEDULER-RS
//!
//! Yet Another JobScheduler
//!
//! # Example
//! ```rust
//! # use tokio_scheduler_rs::job_scheduler::JobScheduler;
//! let scheduler = JobScheduler::default_with_timezone(chrono_tz::PRC);
//! scheduler.register_job(Box::new(HelloWorldJob)).unwrap();
//! scheduler.add_job("HelloWorldJob".into(),"*/5 * * * * * *".into(),None).await.unwrap();
//! scheduler.restore_jobs().await.unwrap(); // This step is used to restore job execute status.
//!                                          // Please notice that you can implement you own job storage to store job status.
//! scheduler.start().await.unwrap(); // `start()` returns a tokio::JoinHandle<()>, you can continue this program if you don't await it.
//! ```

pub mod job_storage;
pub mod errors;
pub mod job;
pub mod job_scheduler;
pub mod job_executor;
pub mod job_hook;
pub use serde_json;
pub use job::ScheduleJob;
pub use job_storage::JobStorage;
pub use job_scheduler::JobScheduler;
pub use job_executor::JobExecutor;
pub use job_storage::MemoryJobStorage;
pub use job_executor::DefaultJobExecutor;
pub use job::JobFuture;
pub use job::WillExecuteJobFuture;
pub use job::JobContext;
pub use serde_json::Value;
pub use async_trait::async_trait;