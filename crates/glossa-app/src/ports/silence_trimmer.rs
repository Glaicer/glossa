use async_trait::async_trait;

use crate::AppError;
use glossa_core::CapturedAudio;

/// Optional silence trimmer applied before transcription.
#[async_trait]
pub trait SilenceTrimmer: Send + Sync {
    async fn trim(&self, input: &CapturedAudio) -> Result<CapturedAudio, AppError>;
}
