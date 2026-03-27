use serde::{Deserialize, Serialize};

/// Supported paste key chords.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PasteMode {
    CtrlV,
    CtrlShiftV,
    ShiftInsert,
}
