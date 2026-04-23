mod audio;
mod input;
mod logging;
mod paste;
mod provider;
mod ui;

use std::env;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

pub use self::audio::{AudioConfig, WorkDir};
pub use self::input::{InputBackend, InputConfig, InputMode};
pub use self::logging::{LogLevel, LoggingConfig};
pub use self::paste::PasteConfig;
pub use self::provider::ProviderConfig;
pub use self::ui::{UiConfig, UiTheme};
use crate::{AppStateKind, CoreError, PasteMode, ProviderKind};

/// TOML-backed Glossa application configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub input: InputConfig,
    pub control: ControlConfig,
    pub provider: ProviderConfig,
    pub audio: AudioConfig,
    pub paste: PasteConfig,
    pub ui: UiConfig,
    pub logging: LoggingConfig,
}

impl AppConfig {
    /// Parses a configuration document from TOML.
    pub fn from_toml_str(input: &str) -> Result<Self, CoreError> {
        let config = toml::from_str::<Self>(input)?;
        config.validate()?;
        Ok(config)
    }

    /// Validates cross-field invariants required by the MVP specification.
    pub fn validate(&self) -> Result<(), CoreError> {
        self.input.validate()?;
        self.control.validate()?;
        self.provider.validate()?;
        self.audio.validate()?;
        self.paste.validate()?;
        self.ui.validate()?;
        self.logging.validate()?;
        Ok(())
    }

    /// Resolves the provider API key based on the configured source.
    pub fn resolve_api_key(&self) -> Result<String, CoreError> {
        self.provider.api_key.resolve()
    }

    /// Builds a minimal runtime status snapshot from static configuration.
    #[must_use]
    pub fn initial_status(&self) -> crate::AppStatus {
        crate::AppStatus {
            state: AppStateKind::Idle,
            provider: self.provider.kind,
            tray_available: self.ui.tray,
            portal_available: self.input.backend == InputBackend::Portal,
        }
    }
}

/// IPC and control settings for the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlConfig {
    pub enable_cli: bool,
    pub socket_path: SocketPath,
}

impl ControlConfig {
    fn validate(&self) -> Result<(), CoreError> {
        if let SocketPath::Custom(path) = &self.socket_path {
            if path.as_str().is_empty() {
                return Err(CoreError::InvalidConfig(
                    "control.socket_path must not be empty".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Socket location for the CLI control channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SocketPath {
    Auto(AutoValue),
    Custom(Utf8PathBuf),
}

impl SocketPath {
    /// Resolves the configured socket path into an absolute runtime location.
    pub fn resolve(&self) -> Result<Utf8PathBuf, CoreError> {
        match self {
            Self::Auto(_) => {
                let runtime_dir = env::var("XDG_RUNTIME_DIR").map_err(|_| {
                    CoreError::InvalidConfig(
                        "XDG_RUNTIME_DIR is required to resolve the runtime socket path".into(),
                    )
                })?;
                Ok(Utf8PathBuf::from(runtime_dir).join("glossa.sock"))
            }
            Self::Custom(path) => Ok(path.clone()),
        }
    }
}

/// Marker used for `"auto"` configuration values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoValue {
    Auto,
}

/// Secret source for API tokens and similar credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum SecretSource {
    Empty,
    Literal(String),
    Env(String),
}

impl SecretSource {
    /// Returns a redacted description suitable for logs and diagnostics.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Empty => "empty".into(),
            Self::Literal(_) => "literal".into(),
            Self::Env(name) => format!("env:{name}"),
        }
    }

    /// Resolves the secret into a string value.
    pub fn resolve(&self) -> Result<String, CoreError> {
        match self {
            Self::Empty => Ok(String::new()),
            Self::Literal(value) => Ok(value.clone()),
            Self::Env(name) => env::var(name).map_err(|_| CoreError::MissingSecret {
                secret_source: format!("env:{name}"),
            }),
        }
    }
}

impl TryFrom<String> for SecretSource {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if let Some(name) = value.strip_prefix("env:") {
            if name.is_empty() {
                return Err(CoreError::InvalidConfig(
                    "provider.api_key env source must not be empty".into(),
                ));
            }
            return Ok(Self::Env(name.to_string()));
        }

        if value.is_empty() {
            return Ok(Self::Empty);
        }

        Ok(Self::Literal(value))
    }
}

impl From<SecretSource> for String {
    fn from(value: SecretSource) -> Self {
        match value {
            SecretSource::Empty => String::new(),
            SecretSource::Literal(inner) => inner,
            SecretSource::Env(inner) => format!("env:{inner}"),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            input: InputConfig::default(),
            control: ControlConfig {
                enable_cli: true,
                socket_path: SocketPath::Auto(AutoValue::Auto),
            },
            provider: ProviderConfig {
                kind: ProviderKind::Groq,
                base_url: Some("https://api.groq.com/openai/v1".into()),
                model: "whisper-large-v3".into(),
                api_key: SecretSource::Env("GROQ_API_KEY".into()),
            },
            audio: AudioConfig::default(),
            paste: PasteConfig {
                mode: PasteMode::CtrlV,
                clipboard_command: "wl-copy".into(),
                type_command: "dotoolc".into(),
            },
            ui: UiConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AudioFormat;

    #[test]
    fn default_config_should_use_dotoolc_for_paste_command() {
        assert_eq!(AppConfig::default().paste.type_command, "dotoolc");
    }

    #[test]
    fn config_should_accept_backend_none() {
        let config = AppConfig {
            input: InputConfig {
                backend: InputBackend::None,
                shortcut: "<Ctrl><Alt>space".into(),
                mode: InputMode::Toggle,
            },
            ..AppConfig::default()
        };

        assert!(config.validate().is_ok());
        assert!(!config.initial_status().portal_available);
    }

    #[test]
    fn openai_compatible_should_require_base_url() {
        let config = AppConfig {
            provider: ProviderConfig {
                kind: ProviderKind::OpenAiCompatible,
                base_url: None,
                model: "whisper-1".into(),
                api_key: SecretSource::Literal("token".into()),
            },
            ..AppConfig::default()
        };

        let error = config.validate().expect_err("validation should fail");
        assert!(error.to_string().contains("base_url"));
    }

    #[test]
    fn secret_source_should_parse_env_values() {
        let source = SecretSource::try_from("env:GROQ_API_KEY".to_string())
            .expect("env-backed secret should parse");
        assert_eq!(source.describe(), "env:GROQ_API_KEY");
    }

    #[test]
    fn secret_source_should_accept_empty_literal() {
        let source = SecretSource::try_from(String::new()).expect("empty secret should parse");
        assert_eq!(source, SecretSource::Empty);
        assert_eq!(source.describe(), "empty");
        assert_eq!(source.resolve().expect("empty secret should resolve"), "");
    }

    #[test]
    fn openai_compatible_should_allow_empty_api_key() {
        let config = AppConfig {
            provider: ProviderConfig {
                kind: ProviderKind::OpenAiCompatible,
                base_url: Some("https://example.com/openai/v1".into()),
                model: "whisper-1".into(),
                api_key: SecretSource::Empty,
            },
            ..AppConfig::default()
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
    fn groq_should_reject_empty_api_key() {
        let config = AppConfig {
            provider: ProviderConfig {
                kind: ProviderKind::Groq,
                base_url: Some("https://api.groq.com/openai/v1".into()),
                model: "whisper-large-v3".into(),
                api_key: SecretSource::Empty,
            },
            ..AppConfig::default()
        };

        let error = config.validate().expect_err("validation should fail");
        assert!(error.to_string().contains("api_key"));
    }

    #[test]
    fn config_should_reject_flac_until_supported() {
        let config = AppConfig {
            audio: AudioConfig {
                format: AudioFormat::Flac,
                ..AudioConfig::default()
            },
            ..AppConfig::default()
        };

        let error = config.validate().expect_err("validation should fail");
        assert!(error.to_string().contains("flac"));
    }

    #[test]
    fn toml_auto_work_dir_should_parse_as_auto_mode() {
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
work_dir = "auto"
format = "wav"
sample_rate_hz = 16000
channels = 1
trim_silence = true
trim_threshold = 500
min_duration_ms = 150
max_duration_sec = 120

[paste]
mode = "ctrl-shift-v"
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

        assert!(matches!(config.audio.work_dir, WorkDir::Auto));
    }
}
