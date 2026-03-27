use serde::{Deserialize, Serialize};

use crate::SessionId;

/// High-level daemon runtime state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppState {
    Idle,
    Recording(RecordingState),
    Processing(ProcessingState),
    Pasting(PastingState),
    ShuttingDown,
}

impl AppState {
    /// Returns the coarse state kind used by status reporting.
    #[must_use]
    pub fn kind(&self) -> AppStateKind {
        match self {
            Self::Idle => AppStateKind::Idle,
            Self::Recording(_) => AppStateKind::Recording,
            Self::Processing(_) => AppStateKind::Processing,
            Self::Pasting(_) => AppStateKind::Pasting,
            Self::ShuttingDown => AppStateKind::ShuttingDown,
        }
    }
}

/// User-visible state categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppStateKind {
    Idle,
    Recording,
    Processing,
    Pasting,
    ShuttingDown,
}

/// Pure metadata for an active recording.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordingState {
    pub session_id: SessionId,
}

/// Pure metadata for a processing cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessingState {
    pub session_id: SessionId,
}

/// Pure metadata for the clipboard/paste phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PastingState {
    pub session_id: SessionId,
    pub text_len: usize,
}
