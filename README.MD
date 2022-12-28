<div style="text-align: center"><h1>TOKIO-SCHEDULER-RS</h1></div>

<div style="text-align: center">Yet Another JobScheduler</div>

<a href="README_CN.MD">简体中文</a>

# Features
* Async Completely
* Witten with tokio runtime
* Maximum Customize

# Example
```rust
use tokio_scheduler_rs::job_scheduler::JobScheduler;

let scheduler = JobScheduler::default_with_timezone(chrono_tz::PRC);
scheduler.register_job(Box::new(HelloWorldJob)).unwrap();
scheduler.add_job("HelloWorldJob".into(),"*/5 * * * * * *".into(),None).await.unwrap();
scheduler.restore_jobs().await.unwrap(); // This step is used to restore job execute status.
                                         // Please notice that you can implement you own job storage to store job status.
scheduler.start().await.unwrap(); // `start()` returns a tokio::JoinHandle<()>, you can continue this program if you don't await it.
```

# License
MIT