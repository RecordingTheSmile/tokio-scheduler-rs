
use std::future::{Future};

use std::pin::Pin;
use std::sync::{Arc, RwLock};




use tokio::task::{JoinHandle};
use crate::job_storage::JobStorage;

/// `JobExecutor` is used to execute jobs
pub trait JobExecutor:Send + Sync{
    /// start job execution
    fn start(&self)->JoinHandle<()>;
    /// stop job execution and wait for all running jobs to complete
    fn stop(&self)->Pin<Box<dyn Future<Output = ()>>>;
}

/// Default implementation for `JobExecutor`
pub struct DefaultJobExecutor<Tz>
where Tz : chrono::TimeZone + Send + Sync,
Tz::Offset: Send + Sync
{
    jobs: Arc<dyn JobStorage<Tz>>,
    tasks:Arc<RwLock<Vec<JoinHandle<()>>>>,
    shutdown_channel:tokio::sync::broadcast::Sender<()>,
    should_next:Arc<RwLock<bool>>
}

impl<Tz> DefaultJobExecutor<Tz>
where Tz : chrono::TimeZone + Send + Sync,
Tz::Offset: Send + Sync
{
    pub fn new(jobs:Arc<dyn JobStorage<Tz>>)->Self{
        let shutdown_chan = tokio::sync::broadcast::channel(1);
        Self{
            jobs,
            tasks:Arc::new(RwLock::new(vec![])),
            shutdown_channel:shutdown_chan.0,
            should_next: Arc::new(RwLock::new(true))
        }
    }
}

impl<Tz> JobExecutor for DefaultJobExecutor<Tz>
where Tz: chrono::TimeZone + Send + Sync + 'static,
Tz::Offset: Send + Sync
{
    fn start(&self) -> JoinHandle<()> {
        let storage = self.jobs.to_owned();
        let shutdown_sender = self.shutdown_channel.to_owned();
        let should_next = self.should_next.to_owned();
        let tasks = self.tasks.to_owned();
        tokio::spawn(async move{
            loop{
                let should_next = match should_next.read(){
                    Ok(v)=>*v,
                    Err(_)=>false
                };

                if !should_next{
                    let _ = shutdown_sender.send(());
                    break;
                }
                let should_exec = match storage.get_all_should_execute_jobs().await{
                    Ok(t)=>t,
                    Err(_)=>continue
                };
                for job in should_exec{
                    let handle = tokio::spawn(async move{
                        job.await;
                        ()
                    });
                    let mut task_vec = match tasks.write(){
                        Ok(v)=>v,
                        Err(_)=>continue
                    };
                    task_vec.push(handle);
                }

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        })
    }

    fn stop(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        let mut shutdown_recv = self.shutdown_channel.subscribe();
        *self.should_next.write().unwrap() = false;
        let tasks = self.tasks.to_owned();
        Box::pin(async move{
            let _ = shutdown_recv.recv().await;
            let tasks = tasks.read().unwrap();
            for i in tasks.iter(){
                loop{
                    if i.is_finished(){
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        })
    }
}