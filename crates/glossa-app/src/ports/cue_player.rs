use async_trait::async_trait;

use crate::AppError;

/// Playback of start/stop cue sounds.
#[async_trait]
pub trait CuePlayer: Send + Sync {
    async fn play_start(&self) -> Result<(), AppError>;
    async fn play_stop(&self) -> Result<(), AppError>;
}
