use async_trait::async_trait;
use camino::Utf8PathBuf;

use crate::AppError;
use glossa_core::{AudioFormat, SessionId};

/// Temporary storage management for capture artifacts.
#[async_trait]
pub trait TempStore: Send + Sync {
    async fn create_recording_path(
        &self,
        session_id: SessionId,
        format: AudioFormat,
    ) -> Result<Utf8PathBuf, AppError>;

    async fn cleanup_session(&self, session_id: SessionId) -> Result<(), AppError>;

    async fn cleanup_stale_files(&self) -> Result<(), AppError>;
}
