use chrono::Local;
use tokio_scheduler_macro::job;
use tokio_scheduler_types::job::{Job, JobContext, JobFuture, JobReturn};

#[job]
pub(crate) async fn example_fn_task(ctx: JobContext) -> anyhow::Result<JobReturn> {
    println!(
        "Hello, World! My JobId is {}, time is: {}",
        ctx.get_id(),
        Local::now()
    );

    Ok(JobReturn::default())
}

#[job]
struct ExampleStructJob;

impl Job for ExampleStructJob {
    fn get_job_name(&self) -> &'static str {
        "ExampleStructJob"
    }

    fn execute(&self, ctx: JobContext) -> JobFuture {
        Box::pin(async move {
            println!(
                "Hello, World! My JobId is {}, time is: {}",
                ctx.get_id(),
                Local::now()
            );
            Ok(JobReturn::default())
        })
    }
}
