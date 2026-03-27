use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::SessionId;

/// Supported on-disk audio formats for captured recordings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioFormat {
    Wav,
    Flac,
}

impl AudioFormat {
    /// Returns the file extension associated with the format.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Flac => "flac",
        }
    }
}

/// Recording parameters used by the audio capture backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordSpec {
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub format: AudioFormat,
    pub max_duration_sec: u32,
}

/// Audio file produced by a completed capture session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedAudio {
    pub session_id: SessionId,
    pub path: Utf8PathBuf,
    pub duration_ms: u64,
    pub sample_rate_hz: u32,
    pub channels: u16,
}
