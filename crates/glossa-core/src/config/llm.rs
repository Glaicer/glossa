use serde::{Deserialize, Serialize};

use super::SecretSource;
use crate::CoreError;

/// LLM enhancement settings for transcription post-processing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmConfig {
    pub enabled: bool,
    pub base_url: String,
    pub model: String,
    pub api_key: SecretSource,
}

impl LlmConfig {
    pub(crate) fn validate(&self) -> Result<(), CoreError> {
        if !self.enabled {
            return Ok(());
        }
        if self.base_url.trim().is_empty() {
            return Err(CoreError::InvalidConfig(
                "llm.base_url must not be empty when enabled".into(),
            ));
        }
        if self.model.trim().is_empty() {
            return Err(CoreError::InvalidConfig(
                "llm.model must not be empty when enabled".into(),
            ));
        }
        Ok(())
    }

    /// Resolves the LLM API key without logging the secret.
    pub fn resolve_api_key(&self) -> Result<String, CoreError> {
        self.api_key.resolve()
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: String::new(),
            model: String::new(),
            api_key: SecretSource::Empty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppConfig;

    #[test]
    fn default_llm_config_should_be_disabled() {
        let config = LlmConfig::default();
        assert!(!config.enabled);
        assert!(config.base_url.is_empty());
        assert!(config.model.is_empty());
        assert_eq!(config.api_key, SecretSource::Empty);
    }

    #[test]
    fn disabled_llm_should_allow_empty_base_url_and_model() {
        let config = LlmConfig {
            enabled: false,
            base_url: String::new(),
            model: String::new(),
            api_key: SecretSource::Empty,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn enabled_llm_should_require_non_empty_base_url() {
        let config = LlmConfig {
            enabled: true,
            base_url: String::new(),
            model: "gpt-4".into(),
            api_key: SecretSource::Empty,
        };
        let error = config.validate().expect_err("validation should fail");
        assert!(error.to_string().contains("base_url"));
    }

    #[test]
    fn enabled_llm_should_require_non_empty_model() {
        let config = LlmConfig {
            enabled: true,
            base_url: "http://localhost:11434/v1".into(),
            model: String::new(),
            api_key: SecretSource::Empty,
        };
        let error = config.validate().expect_err("validation should fail");
        assert!(error.to_string().contains("model"));
    }

    #[test]
    fn enabled_llm_should_accept_empty_api_key() {
        let config = LlmConfig {
            enabled: true,
            base_url: "http://localhost:11434/v1".into(),
            model: "llama3".into(),
            api_key: SecretSource::Empty,
        };
        assert!(config.validate().is_ok());
        assert_eq!(
            config
                .resolve_api_key()
                .expect("empty secret should resolve"),
            ""
        );
    }

    #[test]
    fn app_config_should_accept_no_llm_section() {
        let config = AppConfig::from_toml_str(
            r#"
[input]
backend = "portal"
shortcut = "<Ctrl><Alt>space"
mode = "toggle"

[control]
enable_cli = true
socket_path = "auto"

[provider]
kind = "groq"
base_url = "https://api.groq.com/openai/v1"
model = "whisper-large-v3"
api_key = "env:GROQ_API_KEY"

[audio]
enabled = true
work_dir = "auto"
format = "wav"
sample_rate_hz = 16000
channels = 1
trim_silence = true
trim_threshold = 500
min_duration_ms = 150
max_duration_sec = 120
persist_audio = false
latency_mode = "balanced"
keepalive_after_stop_seconds = 60

[paste]
mode = "ctrl-v"
append_space = false
clipboard_command = "wl-copy"
type_command = "dotoolc"

[ui]
tray = true
idle_icon = "/tmp/idle.png"
recording_icon = "/tmp/recording.png"
start_sound = "/tmp/start.wav"
stop_sound = "/tmp/stop.wav"

[logging]
level = "info"
journal = true
file = false
"#,
        )
        .expect("config should parse");

        assert!(!config.llm.enabled);
        assert!(config.llm.base_url.is_empty());
        assert!(config.llm.model.is_empty());
    }

    #[test]
    fn app_config_should_accept_llm_section() {
        let config = AppConfig::from_toml_str(
            r#"
[input]
backend = "portal"
shortcut = "<Ctrl><Alt>space"
mode = "toggle"

[control]
enable_cli = true
socket_path = "auto"

[provider]
kind = "groq"
base_url = "https://api.groq.com/openai/v1"
model = "whisper-large-v3"
api_key = "env:GROQ_API_KEY"

[audio]
enabled = true
work_dir = "auto"
format = "wav"
sample_rate_hz = 16000
channels = 1
trim_silence = true
trim_threshold = 500
min_duration_ms = 150
max_duration_sec = 120
persist_audio = false
latency_mode = "balanced"
keepalive_after_stop_seconds = 60

[paste]
mode = "ctrl-v"
append_space = false
clipboard_command = "wl-copy"
type_command = "dotoolc"

[ui]
tray = true
idle_icon = "/tmp/idle.png"
recording_icon = "/tmp/recording.png"
start_sound = "/tmp/start.wav"
stop_sound = "/tmp/stop.wav"

[logging]
level = "info"
journal = true
file = false

[LLM]
enabled = true
base_url = "http://localhost:11434/v1"
model = "llama3"
api_key = "env:LLM_API_KEY"
"#,
        )
        .expect("config should parse");

        assert!(config.llm.enabled);
        assert_eq!(config.llm.base_url, "http://localhost:11434/v1");
        assert_eq!(config.llm.model, "llama3");
        assert_eq!(config.llm.api_key.describe(), "env:LLM_API_KEY");
    }
}
