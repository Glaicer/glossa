use serde::{Deserialize, Serialize};

use crate::{CoreError, PasteMode};

/// Clipboard and paste settings for the final text insertion phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasteConfig {
    pub mode: PasteMode,
    #[serde(default)]
    pub append_space: bool,
    pub clipboard_command: String,
    pub type_command: String,
}

impl PasteConfig {
    pub(crate) fn validate(&self) -> Result<(), CoreError> {
        if self.clipboard_command.trim().is_empty() {
            return Err(CoreError::InvalidConfig(
                "paste.clipboard_command must not be empty".into(),
            ));
        }
        if self.type_command.trim().is_empty() {
            return Err(CoreError::InvalidConfig(
                "paste.type_command must not be empty".into(),
            ));
        }
        Ok(())
    }
}
