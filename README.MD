<div style="text-align: center; margin-top: 20px;">
    <img style="max-height: 200px;max-width: 200px;" src="assets/logo.jpg" alt="logo" />
</div>

<div style="text-align: center"><h1>Tokio-Scheduler-Rs</h1></div>

<div style="text-align: center">Yet Another JobScheduler</div>

# Requirements

The 2.0 version of `tokio-scheduler-rs` is designed for Rust version 1.83 and later.

Older version of Rust may work, but it is not tested.

If you get any error, please try to use the 1.0 version for older Rust.

# Features

* Async Completely
* Witten with tokio runtime
* Automatic job register
* Hook support
* Automatic retry support
* Default `tracing` support

# Example

```rust
#[job]
pub(crate) struct ExampleJob;


impl Job for ExampleJob {
    fn get_job_name(&self) -> &'static str {
        "ExampleJob"
    }

    fn execute(&self, ctx: JobContext) -> JobFuture {
        Box::pin(async move {
            println!(
                "Hello, World! My JobId is {}, time is: {}",
                ctx.get_id(),
                chrono::Local::now()
            );
            Ok(JobReturn::default())
        })
    }
}


pub(crate) struct ExampleHook;

impl JobHook for ExampleHook {
    fn before_execute(
        &self,
        _name: &str,
        _id: &str,
        _args: &mut Option<Value>,
        _retry_times: u64,
    ) -> impl Future<Output=JobHookReturn> {
        async move {
            println!("Before execute hook");
            JobHookReturn::NoAction
        }
    }

    fn on_complete(
        &self,
        _name: &str,
        _id: &str,
        _args: &Option<Value>,
        _result: &anyhow::Result<JobReturn>,
        _retry_times: u64,
    ) -> impl Future<Output=JobHookReturn> {
        async move {
            println!("On complete hook");
            JobHookReturn::NoAction
        }
    }

    fn on_success(
        &self,
        _name: &str,
        _id: &str,
        _args: &Option<Value>,
        _return_value: &JobReturn,
        _retry_times: u64,
    ) -> impl Future<Output=JobHookReturn> {
        async move {
            println!("On success hook");
            JobHookReturn::NoAction
        }
    }

    fn on_fail(
        &self,
        _name: &str,
        _id: &str,
        _args: &Option<Value>,
        _error: &anyhow::Error,
        _retry_times: u64,
    ) -> impl Future<Output=JobHookReturn> {
        async move {
            println!("On fail hook");
            JobHookReturn::NoAction
        }
    }
}

#[tokio::main]
async fn main() {
    let producer = DefaultJobProducer::new::<chrono_tz::Tz>(UTC);
    let consumer = DefaultJobConsumer::new();

    let mut opts = JobManagerOptions::default();

    opts.graceful_shutdown_timeout_seconds = 10;
    opts.producer_poll_seconds = 1;

    let job_manager = JobManager::new_with_options(producer, consumer, ExampleHook, opts);

    job_manager.auto_register_job().await.unwrap();

    job_manager
        .schedule_job(ExampleJob, "* * * * * * *", None)
        .await
        .unwrap();

    println!("Start scheduler");
    job_manager.start().await;

    println!("Current Time: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    println!("Stop scheduler");

    job_manager.stop().await;
    println!("Scheduler stopped");
}
```

# Examples

You can see examples in the `examples` directory.

# Contribute

If you have some ideas, you can create a pull request or open an issue.

Any kinds of contributions are welcome!

# License

MIT