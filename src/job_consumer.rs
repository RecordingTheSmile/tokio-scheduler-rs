use crate::errors::{SchedulerError, SchedulerErrorKind};
use crate::JobContext;
use dashmap::DashMap;
use tokio_scheduler_types::job::{Job, JobReturn};
use tokio_scheduler_types::job_consumer::JobConsumer;
use tokio_scheduler_types::job_producer::JobProduceContext;
use tokio_util::sync::CancellationToken;

pub struct DefaultJobConsumer {
    /// JobName -> Job
    jobs: DashMap<&'static str, &'static dyn Job>,
}

impl DefaultJobConsumer {
    pub fn new() -> Self {
        Self {
            jobs: DashMap::new(),
        }
    }
}

impl JobConsumer for DefaultJobConsumer {
    async fn consume(
        &self,
        ctx: JobProduceContext,
        cancellation_token: CancellationToken,
    ) -> anyhow::Result<JobReturn> {
        let job = self.jobs.get(ctx.job_name);

        if let Some(job) = job {
            let run_ctx = JobContext::new(ctx.job_id, ctx.job_args, 0, cancellation_token);
            job.execute(run_ctx).await
        } else {
            anyhow::bail!(SchedulerError::new(SchedulerErrorKind::JobNotExists))
        }
    }

    async fn register_job(&self, job: &'static dyn Job) -> Result<(), SchedulerError> {
        self.jobs.insert(job.get_job_name(), job);
        Ok(())
    }

    async fn unregister_job(&self, job_name: &str) -> Result<(), SchedulerError> {
        self.jobs.remove(job_name);
        Ok(())
    }

    async fn has_job(&self, job_name: &str) -> Result<bool, SchedulerError> {
        Ok(self.jobs.contains_key(job_name))
    }
}
