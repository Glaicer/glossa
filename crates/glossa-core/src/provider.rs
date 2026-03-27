use serde::{Deserialize, Serialize};

/// Configured transcription provider family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    Groq,
    OpenAi,
    OpenAiCompatible,
}

pub use crate::config::ProviderConfig;
