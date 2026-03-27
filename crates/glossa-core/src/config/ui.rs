use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::CoreError;

/// Tray icons and cue sounds used by the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiConfig {
    pub tray: bool,
    pub idle_icon: Utf8PathBuf,
    pub recording_icon: Utf8PathBuf,
    pub processing_icon: Option<Utf8PathBuf>,
    pub start_sound: Utf8PathBuf,
    pub stop_sound: Utf8PathBuf,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            tray: true,
            idle_icon: "/path/to/idle.png".into(),
            recording_icon: "/path/to/recording.png".into(),
            processing_icon: Some("/path/to/processing.png".into()),
            start_sound: "/path/to/start.wav".into(),
            stop_sound: "/path/to/stop.wav".into(),
        }
    }
}

impl UiConfig {
    pub(crate) fn validate(&self) -> Result<(), CoreError> {
        if self.idle_icon.as_str().is_empty() || self.recording_icon.as_str().is_empty() {
            return Err(CoreError::InvalidConfig(
                "ui icons must not be empty".into(),
            ));
        }
        if self.start_sound.as_str().is_empty() || self.stop_sound.as_str().is_empty() {
            return Err(CoreError::InvalidConfig(
                "ui sound paths must not be empty".into(),
            ));
        }
        Ok(())
    }
}
