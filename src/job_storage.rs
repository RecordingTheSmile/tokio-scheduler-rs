use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::{Arc};
use tokio::sync::RwLock;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use cron::Schedule;

use crate::errors::{SchedulerError, SchedulerErrorKind};
use crate::job::{ScheduleJob};


///
/// `JobStorage` is used to store all jobs and context.
/// # Usage
/// `register_job`: Register a job
///
/// `add_job`: Add a job with given cron expression and args
///
/// `delete_job`: Delete a exist job
///
/// `has_job`: Judge is there a job with given JobId
///
/// `get_all_should_execute_jobs`: Get all jobs that should execute now
///
/// `restore_jobs`: Restore all jobs from storage
#[async_trait]
pub trait JobStorage<Tz>: Send + Sync
where Tz:chrono::TimeZone,
Tz::Offset: Send + Sync
{
    ///Register a job
    ///
    /// # Arguments
    /// job: A struct which implements `ScheduleJob` trait.
    ///
    /// # Attention
    /// Every job should provide a unique `job_name` in `get_job_name()` function!
    async fn register_job(&self, job: Box<dyn ScheduleJob>) -> Result<(), SchedulerError>;
    ///Add a job with given cron expression and context.
    ///
    /// # Arguments
    /// 1. `job_name`: A job's name which must be registered before
    ///
    /// 2. `cron`: Cron expression
    ///
    /// 3. `args`: Job args, it's just the alias of `serde_json::Value`
    ///
    /// # Returns
    /// JobId
    async fn add_job(&self, job_name: String, cron: String, args: Option<serde_json::Value>) -> Result<String, SchedulerError>;
    ///Delete a job.
    /// # Arguments
    /// 1. `id`: JobId
    async fn delete_job(&self, id: String) -> Result<(), SchedulerError>;
    /// Judge is there a job with given JobId
    ///
    /// # Arguments
    /// 1. `id`: JobId
    ///
    /// # Returns
    /// A bool which represents the existence of job with given JobId
    async fn has_job(&self, id: String) -> Result<bool, SchedulerError>;
    /// Get all jobs which should execute now.
    ///
    /// # Returns
    /// A vec which store all future which should execute now.
    async fn get_all_should_execute_jobs(&self) -> Result<Vec<Pin<Box<dyn Future<Output = ()> + Send>>>, SchedulerError>;
    /// Restore all jobs from storage
    async fn restore_jobs(&self) -> Result<(), SchedulerError>;
}

/// Simple Memory Job Storage implements `JobStorage`
///
/// !!! This JobStorage is not recommended for production environment !!!
pub struct MemoryJobStorage<Tz = chrono::Utc>
    where Tz: chrono::TimeZone + Sync + Send
{
    tasks: Arc<RwLock<HashMap<String, Box<dyn ScheduleJob>>>>,
    jobs: Arc<RwLock<HashMap<String, (Schedule, String, Option<serde_json::Value>)>>>,
    timezone: Tz,
    last_check_time: Arc<RwLock<DateTime<Tz>>>,
}

impl<Tz> MemoryJobStorage<Tz>
    where Tz: chrono::TimeZone + Send + Sync {
    pub fn new(timezone: Tz) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            timezone: timezone.to_owned(),
            last_check_time: Arc::new(RwLock::new(Local::now().with_timezone(&timezone))),
        }
    }

    pub fn with_timezone(mut self, tz: Tz) -> Self {
        self.timezone = tz;
        self
    }
}

impl Default for MemoryJobStorage<chrono::Utc> {
    fn default() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            timezone: chrono::Utc,
            last_check_time: Arc::new(RwLock::new(Local::now().with_timezone(&chrono::Utc))),
        }
    }
}

unsafe impl<Tz: chrono::TimeZone + Sync + Send> Send for MemoryJobStorage<Tz> {}

unsafe impl<Tz: chrono::TimeZone + Sync + Send> Sync for MemoryJobStorage<Tz> {}

#[async_trait]
impl<Tz> JobStorage<Tz> for MemoryJobStorage<Tz>
where Tz: chrono::TimeZone + Sync + Send,
Tz::Offset: Send+ Sync
{
    async fn register_job(&self, job: Box<dyn ScheduleJob>) -> Result<(), SchedulerError> {
        let is_registered = self.tasks.read().await
            .get(&job.get_job_name())
            .is_some();

        if is_registered {
            return Err(SchedulerError::new(SchedulerErrorKind::JobRegistered));
        }

        self.tasks.write().await
            .insert(job.get_job_name(), job);
        Ok(())
    }

    async fn add_job(&self, job_name: String, cron: String, args: Option<serde_json::Value>) -> Result<String, SchedulerError> {
        let cron = Schedule::from_str(&cron).map_err(|_| SchedulerError::new(SchedulerErrorKind::CronInvalid))?;
        let id = uuid::Uuid::new_v4().to_string();

        self.jobs.write().await
            .insert(id.to_owned(), (cron, job_name, args));

        Ok(id)
    }

    async fn delete_job(&self, id: String) -> Result<(), SchedulerError> {
        self.jobs.write().await
            .remove(&id);

        Ok(())
    }

    async fn has_job(&self, id: String) -> Result<bool, SchedulerError> {
        let has = self.jobs.write().await
            .contains_key(&id);

        Ok(has)
    }

    async fn get_all_should_execute_jobs(&self) -> Result<Vec<Pin<Box<dyn Future<Output = ()> + Send>>>, SchedulerError>
    {
        let time_now = Local::now().with_timezone(&self.timezone);

        let last_check_at = self.last_check_time.read().await;
        let self_jobs = self.jobs.read().await;
        let cron_and_name: Vec<(&Schedule, &String, &Option<serde_json::Value>,&String)> = self_jobs.iter()
            .map(|(id, (cron, task_name, args))| {
                (cron, task_name, args,id)
            }).collect();

        let mut result_vec = vec![];

        let self_tasks = self.tasks.read().await;

        for (cron, name, args,id) in cron_and_name {
            for time in cron.after(&last_check_at) {
                if time <= time_now {
                    match self_tasks.get(name) {
                        Some(v) => result_vec.push(v.execute(id.to_owned(),args.to_owned())),
                        None => break
                    };
                } else {
                    break;
                }
            }
        }

        drop(last_check_at);
        *self.last_check_time.write().await = time_now;

        Ok(result_vec)
    }

    async fn restore_jobs(&self) -> Result<(), SchedulerError> {
        Ok(())
    }

    // fn get_registered_jobs(&self, job_names: &[String]) -> Vec<Pin<Box<dyn Future<Output=()>>>> {
    //     let mut ret = Vec::new();
    //
    //     job_names.iter()
    //         .for_each(|s| {
    //             match self.tasks.read() {
    //                 Ok(v) =>
    //                     match v.get(s) {
    //                         Some(t) => ret.push(t.execute()),
    //                         None => return
    //                     },
    //                 Err(_) => return
    //             };
    //         });
    //
    //     ret
    // }
}