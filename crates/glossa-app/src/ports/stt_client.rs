use async_trait::async_trait;

use crate::AppError;
use glossa_core::CapturedAudio;

/// Speech-to-text client abstraction.
#[async_trait]
pub trait SttClient: Send + Sync {
    fn provider_name(&self) -> &'static str;

    async fn transcribe(&self, audio: &CapturedAudio) -> Result<String, AppError>;
}
