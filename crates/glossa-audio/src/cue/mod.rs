mod rodio_player;

use async_trait::async_trait;
use camino::Utf8PathBuf;

use glossa_app::{ports::CuePlayer, AppError};

pub use self::rodio_player::RodioCuePlayer;

pub enum CuePlayerBackend {
    Rodio(RodioCuePlayer),
    Disabled,
}

impl CuePlayerBackend {
    #[must_use]
    pub fn from_config(enabled: bool, start_sound: Utf8PathBuf, stop_sound: Utf8PathBuf) -> Self {
        if !enabled {
            tracing::info!("cue sounds disabled by config");
            return Self::Disabled;
        }

        Self::Rodio(RodioCuePlayer::new(start_sound, stop_sound))
    }
}

#[async_trait]
impl CuePlayer for CuePlayerBackend {
    async fn play_start(&self) -> Result<(), AppError> {
        match self {
            Self::Rodio(player) => player.play_start().await,
            Self::Disabled => Ok(()),
        }
    }

    async fn play_stop(&self) -> Result<(), AppError> {
        match self {
            Self::Rodio(player) => player.play_stop().await,
            Self::Disabled => Ok(()),
        }
    }
}
