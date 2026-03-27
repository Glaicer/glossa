use thiserror::Error;

/// Shared error type for config/domain validation.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("invalid state transition from {from:?} using {command:?}")]
    InvalidStateTransition {
        from: crate::AppStateKind,
        command: crate::AppCommand,
    },
    #[error("missing secret value from {secret_source}")]
    MissingSecret { secret_source: String },
    #[error("failed to parse TOML configuration: {0}")]
    Toml(#[from] toml::de::Error),
}
