use crate::errors::{SchedulerError, SchedulerErrorKind};
use chrono::{DateTime, TimeZone, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde_json::Value;
use tokio_scheduler_types::job_producer::{JobProduceContext, JobProducer};

pub struct DefaultJobProducer<Tz>
where
    Tz: TimeZone,
{
    /// JobId -> (JobExecContext, Cron)
    jobs: DashMap<String, (JobProduceContext, cron::Schedule)>,
    timezone: Tz,
    last_checked_at: RwLock<DateTime<Tz>>,
    retry_jobs: RwLock<Vec<JobProduceContext>>,
}

impl<Tz> DefaultJobProducer<Tz>
where
    Tz: TimeZone,
{
    pub fn new<T>(timezone: Tz) -> Self {
        Self {
            jobs: DashMap::new(),
            timezone: timezone.to_owned(),
            last_checked_at: RwLock::new(Utc::now().with_timezone(&timezone)),
            retry_jobs: RwLock::new(Vec::new()),
        }
    }
}

impl<Tz> JobProducer for DefaultJobProducer<Tz>
where
    Tz: TimeZone + Send + Sync,
    Tz::Offset: Send + Sync,
{
    async fn fetch_jobs_to_execute(&self) -> Vec<JobProduceContext> {
        let now = Utc::now().with_timezone(&self.timezone);

        let mut ret = vec![];

        for job in &self.jobs {
            if let Some(next_exec_at) = job.1.after(&self.last_checked_at.read()).next() {
                if next_exec_at <= now {
                    ret.push(job.0.clone());
                }
            }
        }

        *self.last_checked_at.write() = now;

        let mut retry_jobs = self.retry_jobs.write();

        while let Some(retry_jobs) = retry_jobs.pop() {
            ret.push(retry_jobs);
        }
        ret
    }

    async fn reschedule_retry_job(&self, mut ctx: JobProduceContext) -> Result<(), SchedulerError> {
        ctx.retry_times += 1;
        self.retry_jobs.write().push(ctx);
        Ok(())
    }

    async fn register_job(
        &self,
        job_name: &'static str,
        execute_at_cron: &str,
        args: Option<Value>,
    ) -> Result<String, SchedulerError> {
        let job_id = uuid::Uuid::new_v4().to_string();

        let cron = execute_at_cron.parse().map_err(|e| {
            SchedulerError::new(SchedulerErrorKind::CronInvalid(
                execute_at_cron.to_owned(),
                e,
            ))
        })?;

        self.jobs.insert(
            job_id.to_owned(),
            (
                JobProduceContext {
                    job_name,
                    job_id: job_id.to_owned(),
                    job_args: args.to_owned(),
                    retry_times: 0,
                },
                cron,
            ),
        );

        Ok(job_id)
    }

    async fn unregister_job(&self, job_id: &str) -> Result<Option<&'static str>, SchedulerError> {
        if let Some((_, job)) = self.jobs.remove(job_id) {
            Ok(Some(job.0.job_name))
        } else {
            Ok(None)
        }
    }

    async fn unregister_job_by_name(&self, job_name: &'static str) -> Result<(), SchedulerError> {
        self.jobs.retain(|_, (ctx, _)| ctx.job_name != job_name);

        Ok(())
    }

    async fn has_job(&self, job_id: &str) -> Result<bool, SchedulerError> {
        Ok(self.jobs.contains_key(job_id))
    }

    async fn is_job_scheduled(&self, job_name: &str) -> Result<bool, SchedulerError> {
        Ok(self
            .jobs
            .iter()
            .filter(|v| v.value().0.job_name == job_name)
            .next()
            .is_some())
    }
}
