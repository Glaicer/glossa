use std::io;

use thiserror::Error;

use glossa_core::CoreError;

/// Application-layer error spanning orchestration and adapter failures.
#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("{context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("{0}")]
    Message(String),
}

impl AppError {
    /// Creates an opaque application error message.
    #[must_use]
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }

    /// Wraps an I/O error with human-readable context.
    #[must_use]
    pub fn io(context: &'static str, source: io::Error) -> Self {
        Self::Io { context, source }
    }
}
