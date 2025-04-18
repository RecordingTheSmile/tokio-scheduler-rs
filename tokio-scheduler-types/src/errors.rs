use anyhow::Error;
use std::fmt::{Debug, Display, Formatter};

/// Error kind for scheduler
#[derive(Debug)]
pub enum SchedulerErrorKind {
    /// Cron expression is invalid
    CronInvalid(String, cron::error::Error),
    /// Job not exists
    JobNotExists,
    /// Other errors
    CustomErr(Error),
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct SchedulerError {
    error_kind: SchedulerErrorKind,
}

impl SchedulerError {
    pub fn new(error_kind: SchedulerErrorKind) -> Self {
        Self { error_kind }
    }
}

impl Display for SchedulerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}

impl std::error::Error for SchedulerError {}

impl Into<SchedulerError> for SchedulerErrorKind {
    fn into(self) -> SchedulerError {
        SchedulerError::new(self)
    }
}

impl From<anyhow::Error> for SchedulerErrorKind {
    fn from(value: Error) -> Self {
        Self::CustomErr(value)
    }
}

impl From<anyhow::Error> for SchedulerError {
    fn from(value: Error) -> Self {
        Self::new(SchedulerErrorKind::from(value))
    }
}
