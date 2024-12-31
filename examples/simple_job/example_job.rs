use tokio_scheduler_rs::{Job, JobContext};
use tokio_scheduler_types::job::{JobFuture, JobReturn};

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
