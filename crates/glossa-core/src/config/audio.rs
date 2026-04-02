use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{AudioFormat, CoreError};

/// Audio capture and processing settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioConfig {
    pub work_dir: WorkDir,
    pub format: AudioFormat,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub trim_silence: bool,
    pub trim_threshold: u16,
    pub min_duration_ms: u64,
    pub max_duration_sec: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            work_dir: WorkDir::Auto,
            format: AudioFormat::Wav,
            sample_rate_hz: 16_000,
            channels: 1,
            trim_silence: true,
            trim_threshold: 500,
            min_duration_ms: 150,
            max_duration_sec: 120,
        }
    }
}

impl AudioConfig {
    pub(crate) fn validate(&self) -> Result<(), CoreError> {
        if self.sample_rate_hz == 0 {
            return Err(CoreError::InvalidConfig(
                "audio.sample_rate_hz must be greater than zero".into(),
            ));
        }
        if self.channels == 0 {
            return Err(CoreError::InvalidConfig(
                "audio.channels must be greater than zero".into(),
            ));
        }
        if self.min_duration_ms == 0 {
            return Err(CoreError::InvalidConfig(
                "audio.min_duration_ms must be greater than zero".into(),
            ));
        }
        if self.max_duration_sec == 0 {
            return Err(CoreError::InvalidConfig(
                "audio.max_duration_sec must be greater than zero".into(),
            ));
        }
        if self.format == AudioFormat::Flac {
            return Err(CoreError::InvalidConfig(
                "audio.format flac is not implemented yet; use wav for the MVP".into(),
            ));
        }
        Ok(())
    }
}

/// Working directory selection for temporary recordings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum WorkDir {
    Auto,
    Path(Utf8PathBuf),
}

impl TryFrom<String> for WorkDir {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == "auto" {
            Ok(Self::Auto)
        } else if value.trim().is_empty() {
            Err("audio.work_dir must not be empty".into())
        } else {
            Ok(Self::Path(Utf8PathBuf::from(value)))
        }
    }
}

impl From<WorkDir> for String {
    fn from(value: WorkDir) -> Self {
        match value {
            WorkDir::Auto => "auto".into(),
            WorkDir::Path(path) => path.into_string(),
        }
    }
}
