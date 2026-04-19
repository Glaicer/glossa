use std::{collections::BTreeSet, fs, path::Path};

use glossa_app::AppError;
use glossa_core::{AppConfig, InputBackend, InputMode, PasteMode, ProviderKind, UiTheme};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SettingsValues {
    pub(super) input_backend: InputBackend,
    pub(super) input_mode: InputMode,
    pub(super) enable_cli: bool,
    pub(super) provider_kind: ProviderKind,
    pub(super) provider_base_url: String,
    pub(super) provider_model: String,
    pub(super) provider_api_key: String,
    pub(super) paste_mode: PasteMode,
    pub(super) ui_theme: UiTheme,
}

impl SettingsValues {
    pub(super) fn from_config(config: &AppConfig) -> Self {
        Self {
            input_backend: config.input.backend,
            input_mode: config.input.mode,
            enable_cli: config.control.enable_cli,
            provider_kind: config.provider.kind,
            provider_base_url: config.provider.base_url.clone().unwrap_or_default(),
            provider_model: config.provider.model.clone(),
            provider_api_key: String::from(config.provider.api_key.clone()),
            paste_mode: config.paste.mode,
            ui_theme: config.ui.theme,
        }
    }
}

pub(super) fn apply_settings_to_config(
    source: &str,
    settings: &SettingsValues,
) -> Result<String, AppError> {
    let updates = build_updates(settings);
    let mut seen = BTreeSet::new();
    let mut current_section = None::<&str>;
    let mut updated = String::with_capacity(source.len() + 128);

    for raw_line in lines_with_endings(source) {
        let (line, newline) = split_line_ending(raw_line);

        if let Some(section) = parse_section_name(line) {
            current_section = Some(section);
            updated.push_str(line);
            updated.push_str(newline);
            continue;
        }

        let replacement = current_section.and_then(|section| {
            updates
                .iter()
                .find(|update| update.section == section && line_matches_key(line, update.key))
        });

        if let Some(update) = replacement {
            updated.push_str(&replace_line_value(line, &update.value));
            updated.push_str(newline);
            seen.insert((update.section, update.key));
            continue;
        }

        updated.push_str(line);
        updated.push_str(newline);
    }

    let missing: Vec<String> = updates
        .iter()
        .filter(|update| !seen.contains(&(update.section, update.key)))
        .map(|update| format!("[{}].{}", update.section, update.key))
        .collect();
    if !missing.is_empty() {
        return Err(AppError::message(format!(
            "config.toml is missing required setting lines: {}",
            missing.join(", ")
        )));
    }

    AppConfig::from_toml_str(&updated)?;
    Ok(updated)
}

pub(super) fn write_settings_to_config(
    path: &Path,
    settings: &SettingsValues,
) -> Result<(), AppError> {
    let source = fs::read_to_string(path)
        .map_err(|error| AppError::io("failed to read config.toml", error))?;
    let updated = apply_settings_to_config(&source, settings)?;
    fs::write(path, updated).map_err(|error| AppError::io("failed to write config.toml", error))
}

pub(super) fn input_backend_id(value: InputBackend) -> &'static str {
    match value {
        InputBackend::Portal => "portal",
        InputBackend::None => "none",
    }
}

pub(super) fn parse_input_backend(value: &str) -> Option<InputBackend> {
    match value {
        "portal" => Some(InputBackend::Portal),
        "none" => Some(InputBackend::None),
        _ => None,
    }
}

pub(super) fn input_mode_id(value: InputMode) -> &'static str {
    match value {
        InputMode::PushToTalk => "push-to-talk",
        InputMode::Toggle => "toggle",
    }
}

pub(super) fn parse_input_mode(value: &str) -> Option<InputMode> {
    match value {
        "push-to-talk" => Some(InputMode::PushToTalk),
        "toggle" => Some(InputMode::Toggle),
        _ => None,
    }
}

pub(super) fn provider_kind_id(value: ProviderKind) -> &'static str {
    match value {
        ProviderKind::Groq => "groq",
        ProviderKind::OpenAi => "openai",
        ProviderKind::OpenAiCompatible => "openai-compatible",
    }
}

pub(super) fn parse_provider_kind(value: &str) -> Option<ProviderKind> {
    match value {
        "groq" => Some(ProviderKind::Groq),
        "openai" => Some(ProviderKind::OpenAi),
        "openai-compatible" => Some(ProviderKind::OpenAiCompatible),
        _ => None,
    }
}

pub(super) fn paste_mode_id(value: PasteMode) -> &'static str {
    match value {
        PasteMode::CtrlV => "ctrl-v",
        PasteMode::CtrlShiftV => "ctrl-shift-v",
        PasteMode::ShiftInsert => "shift-insert",
    }
}

pub(super) fn parse_paste_mode(value: &str) -> Option<PasteMode> {
    match value {
        "ctrl-v" => Some(PasteMode::CtrlV),
        "ctrl-shift-v" => Some(PasteMode::CtrlShiftV),
        "shift-insert" => Some(PasteMode::ShiftInsert),
        _ => None,
    }
}

pub(super) fn ui_theme_id(value: UiTheme) -> &'static str {
    match value {
        UiTheme::Dark => "dark",
        UiTheme::Light => "light",
    }
}

pub(super) fn parse_ui_theme(value: &str) -> Option<UiTheme> {
    match value {
        "dark" => Some(UiTheme::Dark),
        "light" => Some(UiTheme::Light),
        _ => None,
    }
}

fn build_updates(settings: &SettingsValues) -> [SettingUpdate; 9] {
    [
        SettingUpdate::new(
            "input",
            "backend",
            quoted(input_backend_id(settings.input_backend)),
        ),
        SettingUpdate::new("input", "mode", quoted(input_mode_id(settings.input_mode))),
        SettingUpdate::new("control", "enable_cli", settings.enable_cli.to_string()),
        SettingUpdate::new(
            "provider",
            "kind",
            quoted(provider_kind_id(settings.provider_kind)),
        ),
        SettingUpdate::new("provider", "base_url", quoted(&settings.provider_base_url)),
        SettingUpdate::new("provider", "model", quoted(&settings.provider_model)),
        SettingUpdate::new("provider", "api_key", quoted(&settings.provider_api_key)),
        SettingUpdate::new("paste", "mode", quoted(paste_mode_id(settings.paste_mode))),
        SettingUpdate::new("ui", "theme", quoted(ui_theme_id(settings.ui_theme))),
    ]
}

fn quoted(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');

    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0C}' => escaped.push_str("\\f"),
            _ => escaped.push(ch),
        }
    }

    escaped.push('"');
    escaped
}

fn lines_with_endings(input: &str) -> Vec<&str> {
    if input.is_empty() {
        return Vec::new();
    }

    input.split_inclusive('\n').collect()
}

fn split_line_ending(line: &str) -> (&str, &str) {
    line.strip_suffix('\n')
        .map_or((line, ""), |line| (line, "\n"))
}

fn parse_section_name(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        return None;
    }

    trimmed.strip_prefix('[')?.strip_suffix(']')
}

fn line_matches_key(line: &str, key: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return false;
    }

    let Some(eq_index) = trimmed.find('=') else {
        return false;
    };

    trimmed[..eq_index].trim_end() == key
}

fn replace_line_value(line: &str, new_value: &str) -> String {
    let trimmed = line.trim_start();
    let leading_offset = line.len() - trimmed.len();
    let eq_index = leading_offset
        + trimmed
            .find('=')
            .expect("matched key lines must contain '='");
    let prefix = &line[..=eq_index];
    let after_eq = &line[eq_index + 1..];
    let comment_index = find_comment_start(after_eq).unwrap_or(after_eq.len());
    let value_segment = &after_eq[..comment_index];
    let leading_ws_len =
        value_segment.len() - value_segment.trim_start_matches(char::is_whitespace).len();
    let trailing_ws_len =
        value_segment.len() - value_segment.trim_end_matches(char::is_whitespace).len();
    let leading_ws = &value_segment[..leading_ws_len];
    let trailing_ws = &value_segment[value_segment.len() - trailing_ws_len..];
    let suffix = &after_eq[comment_index..];

    format!("{prefix}{leading_ws}{new_value}{trailing_ws}{suffix}")
}

fn find_comment_start(value: &str) -> Option<usize> {
    let mut in_basic_string = false;
    let mut in_literal_string = false;
    let mut escape_next = false;

    for (index, ch) in value.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_basic_string => escape_next = true,
            '"' if !in_literal_string => in_basic_string = !in_basic_string,
            '\'' if !in_basic_string => in_literal_string = !in_literal_string,
            '#' if !in_basic_string && !in_literal_string => return Some(index),
            _ => {}
        }
    }

    None
}

struct SettingUpdate {
    section: &'static str,
    key: &'static str,
    value: String,
}

impl SettingUpdate {
    fn new(section: &'static str, key: &'static str, value: String) -> Self {
        Self {
            section,
            key,
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use glossa_core::{InputBackend, InputMode, PasteMode, ProviderKind, UiTheme};

    use super::{apply_settings_to_config, quoted, SettingsValues};

    fn valid_config_source() -> String {
        r#"[input]
# Backend comment
backend = "portal"
mode = "push-to-talk"
shortcut = "<Ctrl><Alt>space"

[control]
enable_cli = true # keep this comment
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
mode = "ctrl-v"
clipboard_command = "wl-copy"
type_command = "dotool"

[ui]
theme = "dark"
tray = true
idle_icon = "/tmp/idle.png"
recording_icon = "/tmp/recording.png"
processing_icon = "/tmp/processing.png"
idle_dark_icon = "/tmp/idle_dark.png"
recording_dark_icon = "/tmp/recording_dark.png"
processing_dark_icon = "/tmp/processing_dark.png"
start_sound = "/tmp/start.wav"
stop_sound = "/tmp/stop.wav"

[logging]
level = "info"
journal = true
file = false
"#
        .into()
    }

    fn updated_settings() -> SettingsValues {
        SettingsValues {
            input_backend: InputBackend::None,
            input_mode: InputMode::Toggle,
            enable_cli: false,
            provider_kind: ProviderKind::OpenAiCompatible,
            provider_base_url: "https://example.com/v1".into(),
            provider_model: "custom-model".into(),
            provider_api_key: "env:CUSTOM_KEY".into(),
            paste_mode: PasteMode::ShiftInsert,
            ui_theme: UiTheme::Light,
        }
    }

    #[test]
    fn apply_settings_to_config_should_update_only_supported_lines_and_preserve_comments() {
        let updated = apply_settings_to_config(&valid_config_source(), &updated_settings())
            .expect("config patch should succeed");

        let expected = r#"[input]
# Backend comment
backend = "none"
mode = "toggle"
shortcut = "<Ctrl><Alt>space"

[control]
enable_cli = false # keep this comment
socket_path = "auto"

[provider]
kind = "openai-compatible"
base_url = "https://example.com/v1"
model = "custom-model"
api_key = "env:CUSTOM_KEY"

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
mode = "shift-insert"
clipboard_command = "wl-copy"
type_command = "dotool"

[ui]
theme = "light"
tray = true
idle_icon = "/tmp/idle.png"
recording_icon = "/tmp/recording.png"
processing_icon = "/tmp/processing.png"
idle_dark_icon = "/tmp/idle_dark.png"
recording_dark_icon = "/tmp/recording_dark.png"
processing_dark_icon = "/tmp/processing_dark.png"
start_sound = "/tmp/start.wav"
stop_sound = "/tmp/stop.wav"

[logging]
level = "info"
journal = true
file = false
"#;

        assert_eq!(updated, expected);
    }

    #[test]
    fn apply_settings_to_config_should_fail_when_a_supported_key_is_missing() {
        let source = valid_config_source().replace("mode = \"push-to-talk\"\n", "");

        let error = apply_settings_to_config(&source, &updated_settings())
            .expect_err("missing mode line should fail");

        assert!(error.to_string().contains("[input].mode"));
    }

    #[test]
    fn quoted_should_escape_backslashes_and_quotes() {
        let quoted = quoted(r#"env:\"quoted\"\key"#);

        assert_eq!(quoted, r#""env:\\\"quoted\\\"\\key""#);
    }
}
