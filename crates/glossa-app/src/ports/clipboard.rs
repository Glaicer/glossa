use async_trait::async_trait;

use crate::AppError;

/// Clipboard writer used for the final transcription text.
#[async_trait]
pub trait ClipboardWriter: Send + Sync {
    async fn set_text(&self, text: &str) -> Result<(), AppError>;
}
