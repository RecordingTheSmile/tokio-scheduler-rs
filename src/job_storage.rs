use std::collections::HashMap;
use std::future::Future;
use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use crate::errors::{SchedulerError, SchedulerErrorKind};
use crate::types::PinFutureResult;

pub trait JobStorage{
    fn add_job<Job>(&mut self,job:Job)-> Result<String,SchedulerError>
    where Job:Future<Output = ()> + Send + 'static
    ;
    fn delete_job(&self,id:String)->Result<(),SchedulerError>;
    fn has_job(&self,id:String)->Result<bool,SchedulerError>;
    fn get_all_should_execute_jobs(&self)->Result<Vec<Pin<Box<dyn Future<Output = ()>>>>,SchedulerError>;
}

pub struct MemoryJobStorage{
    jobs:Arc<RwLock<HashMap<String,Pin<Box<dyn Future<Output = ()>>>>>>
}

impl JobStorage for MemoryJobStorage{
    fn add_job<Job>(&mut self, job: Job) -> Result<String, SchedulerError> where Job: Future<Output=()> + Send + 'static {
        let pinned_job = Box::pin(job);
        let id = uuid::Uuid::new_v4().to_string();
        self.jobs.write().unwrap().insert(id.to_owned(), pinned_job);
        Ok(id)
    }

    fn delete_job(&self, id: String) -> Result<(), SchedulerError> {
        todo!()
    }

    fn has_job(&self, id: String) -> Result<bool, SchedulerError> {
        todo!()
    }

    fn get_all_should_execute_jobs(&self) -> Result<Vec<Pin<Box<dyn Future<Output=()>>>>, SchedulerError> {
        todo!()
    }
}