use std::future::Future;
use std::pin::Pin;
use serde_json::Value;

/// All jobs should implements this trait
///
/// # Example
/// ```rust
/// # use std::future::Future;
/// # use std::pin::Pin;
/// # use tokio_scheduler_rs::job::ScheduleJob;
/// pub struct TestJob;
///
///     impl ScheduleJob for TestJob{
///         fn get_job_name(&self) -> String {
///             String::from("TestJob")
///         }
///
///         fn execute(&self,id:String,_:Option<serde_json::Value>) -> Pin<Box<dyn Future<Output=()>>> {
///             Box::pin(async move{
///                 println!("Hello,World! My Task Uuid is: {}",id);
///             })
///         }
///    }
/// ```
/// # Attention
/// `job_name` must be unique!!!

pub trait ScheduleJob:Send + Sync{
    fn get_job_name(&self)->String;
    fn execute(&self,id:String,args:Option<Value>)->Pin<Box<dyn Future<Output = ()> + Send + Sync>>;
}