<div style="text-align: center"><h1>TOKIO-SCHEDULER-RS</h1></div>

<div style="text-align: center">基于Tokio的定时任务库</div>

<a href="README.MD">English</a>

# 特点
* 完全异步
* 基于Tokio编写，完全兼容Tokio生态
* 高度自定义性

# 例子
```rust
use tokio_scheduler_rs::job_scheduler::JobScheduler;

let scheduler = JobScheduler::default_with_timezone(chrono_tz::PRC);
scheduler.register_job(Box::new(HelloWorldJob)).unwrap();
scheduler.add_job("HelloWorldJob".into(),"*/5 * * * * * *".into(),None).await.unwrap();
scheduler.restore_jobs().await.unwrap(); // 这一步用来从任务存储中还原上下文，通常在程序重启后恢复执行
                                         // 你可以自己实现`JobStorage` trait 来编写自己的任务存储
scheduler.start().await.unwrap(); // `start()` 返回一个 tokio::JoinHandle<()> , 如果你想继续执行，可以忽略这个返回值
```

# 许可证
MIT