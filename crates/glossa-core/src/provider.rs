use serde::{Deserialize, Serialize};

/// Configured transcription provider family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    #[serde(rename = "groq")]
    Groq,
    #[serde(rename = "openai", alias = "open-ai")]
    OpenAi,
    #[serde(rename = "openai-compatible", alias = "open-ai-compatible")]
    OpenAiCompatible,
}

pub use crate::config::ProviderConfig;

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::ProviderKind;

    #[derive(Deserialize)]
    struct ProviderKindDocument {
        kind: ProviderKind,
    }

    #[test]
    fn provider_kind_should_accept_documented_openai_values() {
        let document: ProviderKindDocument =
            toml::from_str("kind = \"openai-compatible\"").expect("documented value should parse");

        assert_eq!(document.kind, ProviderKind::OpenAiCompatible);
    }

    #[test]
    fn provider_kind_should_accept_legacy_kebab_case_aliases() {
        let document: ProviderKindDocument =
            toml::from_str("kind = \"open-ai\"").expect("legacy alias should parse");

        assert_eq!(document.kind, ProviderKind::OpenAi);
    }

    #[test]
    fn provider_kind_should_accept_groq() {
        let document: ProviderKindDocument =
            toml::from_str("kind = \"groq\"").expect("groq should parse");

        assert_eq!(document.kind, ProviderKind::Groq);
    }
}
