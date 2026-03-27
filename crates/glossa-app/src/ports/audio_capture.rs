use async_trait::async_trait;
use camino::Utf8Path;

use crate::AppError;
use glossa_core::{CapturedAudio, RecordSpec, SessionId};

/// Audio capture backend capable of starting a new recording session.
#[async_trait]
pub trait AudioCapture: Send + Sync {
    async fn start(
        &self,
        session_id: SessionId,
        spec: RecordSpec,
        path: &Utf8Path,
    ) -> Result<Box<dyn ActiveRecording>, AppError>;
}

/// Active recording handle owned by the daemon while capture is running.
#[async_trait(?Send)]
pub trait ActiveRecording {
    async fn stop(self: Box<Self>) -> Result<CapturedAudio, AppError>;
    async fn abort(self: Box<Self>) -> Result<(), AppError>;
}
