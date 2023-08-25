use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait JobHook: Send + Sync {
    async fn on_execute(&self, name: &str, id: &str, args: &Option<Value>) -> JobHookReturn;
    async fn on_complete(
        &self,
        name: &str,
        id: &str,
        args: &Option<Value>,
        result: &anyhow::Result<Value>,
        retry_times: u64,
    ) -> JobHookReturn;
    async fn on_success(
        &self,
        name: &str,
        id: &str,
        args: &Option<Value>,
        return_vaule: &Value,
        retry_times: u64,
    ) -> JobHookReturn;
    async fn on_fail(
        &self,
        name: &str,
        id: &str,
        args: &Option<Value>,
        error: &anyhow::Error,
        retry_times: u64,
    ) -> JobHookReturn;
}

#[derive(PartialEq, Eq)]
pub enum JobHookReturn {
    NoAction,
    CancelRunning,
    RemoveJob,
    RetryJob,
}
