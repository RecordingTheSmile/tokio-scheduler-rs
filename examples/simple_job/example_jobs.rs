use tokio_scheduler_rs::{JobFuture, ScheduleJob, Value};

pub struct ExampleJob;

impl ExampleJob {
    // You can define a unique job name here, which can avoid copy and paste them everywhere in program.
    pub const JOB_NAME: &'static str = "HelloWorldJob";
}

impl ScheduleJob for ExampleJob {
    // define a unique `job_name` here!
    fn get_job_name(&self) -> String {
        String::from(Self::JOB_NAME)
    }

    fn execute(&self, ctx: tokio_scheduler_rs::job::JobContext) -> JobFuture {
        Box::pin(async move {
            println!("Hello,World! Job context is: {:#?}", ctx);

            Ok(Value::default())
        })
    }
}
