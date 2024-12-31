#![forbid(unsafe_code)]
#![doc=include_str!("../README.MD")]
pub use tokio_scheduler_types::errors;
pub mod job_consumer;
pub mod job_hook;
pub mod job_manager;
pub mod job_producer;
pub use tokio_scheduler_types as types;

pub use anyhow::anyhow as anyhow_error;
pub use anyhow::bail;
pub use anyhow::Error;
pub use anyhow::Result;
pub use async_trait::async_trait;
pub use cron::Schedule as CronExpression;
pub use inventory;
pub use job_consumer::DefaultJobConsumer;
pub use job_hook::EmptyHook;
pub use job_manager::{JobManager, JobManagerOptions};
pub use job_producer::DefaultJobProducer;
pub use serde_json;
pub use serde_json::Value;
pub use tokio_scheduler_macro as macros;
pub use tokio_scheduler_types::job::Job;
pub use tokio_scheduler_types::job::JobContext;
pub use types::job;
pub use types::job_consumer as job_consumer_types;
pub use types::job_hook as job_hook_types;
pub use types::job_hook::JobHook;
pub use types::job_hook::JobHookReturn;
pub use types::job_producer as job_producer_types;
pub use types::job_producer::JobProduceContext;
