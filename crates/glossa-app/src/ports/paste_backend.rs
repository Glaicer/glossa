use async_trait::async_trait;

use crate::AppError;
use glossa_core::PasteMode;

/// Input emulation backend used to paste clipboard contents.
#[async_trait]
pub trait PasteBackend: Send + Sync {
    async fn paste(&self, mode: PasteMode) -> Result<(), AppError>;
}
