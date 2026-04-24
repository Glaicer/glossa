use async_trait::async_trait;
use camino::Utf8Path;
use std::time::Duration;

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

    async fn ensure_idle_stream_on(&self) -> Result<(), AppError>;
    async fn ensure_idle_stream_off(&self) -> Result<(), AppError>;
    async fn schedule_idle_stream_timeout(&self, timeout: Duration) -> Result<(), AppError>;
    async fn is_idle_stream_active(&self) -> bool;
}

/// Active recording handle owned by the daemon while capture is running.
#[async_trait(?Send)]
pub trait ActiveRecording {
    async fn stop(self: Box<Self>) -> Result<CapturedAudio, AppError>;
    async fn abort(self: Box<Self>) -> Result<(), AppError>;
}
