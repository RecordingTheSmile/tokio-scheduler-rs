use tokio_scheduler_rs::{ScheduleJob, Value};

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

    fn execute(
        &self,
        ctx: &mut tokio_scheduler_rs::job::JobContext,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = tokio_scheduler_rs::Result<serde_json::Value>> + Send>,
    > {
        Box::pin(async move {
            println!("Hello,World! Job context is: {:#?}", ctx);

            Ok(Value::default())
        })
    }
}
