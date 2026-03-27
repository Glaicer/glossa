use async_trait::async_trait;

use crate::AppError;

/// Tray UI state reflected to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    Idle,
    Recording,
    Processing,
}

/// Best-effort tray integration.
#[async_trait]
pub trait TrayPort: Send + Sync {
    async fn set_state(&self, state: TrayState) -> Result<(), AppError>;
    async fn show_error(&self, message: &str) -> Result<(), AppError>;
}

/// No-op tray implementation used when the environment does not support a tray.
#[derive(Debug, Default)]
pub struct NullTrayPort;

#[async_trait]
impl TrayPort for NullTrayPort {
    async fn set_state(&self, _state: TrayState) -> Result<(), AppError> {
        Ok(())
    }

    async fn show_error(&self, _message: &str) -> Result<(), AppError> {
        Ok(())
    }
}
