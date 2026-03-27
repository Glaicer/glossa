use serde::{Deserialize, Serialize};

use crate::CoreError;

/// Log level used by the daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Logging configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub journal: bool,
    pub file: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            journal: true,
            file: false,
        }
    }
}

impl LoggingConfig {
    pub(crate) fn validate(&self) -> Result<(), CoreError> {
        if !self.journal && !self.file {
            return Err(CoreError::InvalidConfig(
                "logging must enable at least one sink".into(),
            ));
        }
        Ok(())
    }
}
