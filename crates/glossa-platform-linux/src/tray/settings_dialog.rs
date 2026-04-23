use gtk::{
    glib::object::IsA, prelude::*, Box as GtkBox, CheckButton, ComboBoxText, Dialog, DialogFlags,
    Entry, Frame, Grid, Label, Orientation, ResponseType, Widget, Window,
};

use glossa_app::AppError;

use super::settings::{
    input_backend_id, input_mode_id, parse_input_backend, parse_input_mode, parse_paste_mode,
    parse_provider_kind, parse_ui_theme, paste_mode_id, provider_kind_id, ui_theme_id,
    SettingsValues,
};

const INPUT_BACKEND_TOOLTIP: &str =
    "Selects the input backend that listens for recording commands. Options: portal or none.";
const INPUT_MODE_TOOLTIP: &str =
    "Selects how the portal shortcut behaves. Options: push-to-talk or toggle.";
const ENABLE_CLI_TOOLTIP: &str =
    "Enables the CLI control channel used by commands like `glossa ctl toggle`.";
const PROVIDER_KIND_TOOLTIP: &str =
    "Selects the speech-to-text provider mode. Options: groq, openai, or openai-compatible.";
const PROVIDER_BASE_URL_TOOLTIP: &str =
    "Sets the provider base URL. Required for openai-compatible configurations.";
const PROVIDER_MODEL_TOOLTIP: &str =
    "Sets the transcription model that the configured provider should use.";
const PROVIDER_API_KEY_TOOLTIP: &str =
    "Sets the provider API key source or literal secret. Values such as env:VAR are supported.";
const PASTE_MODE_TOOLTIP: &str = "Selects which keyboard shortcut dotool should emulate for paste.";
const APPEND_SPACE_TOOLTIP: &str =
    "Appends one trailing space to the pasted transcription for continuous dictation.";
const UI_THEME_TOOLTIP: &str = "Selects which tray icon set to use. Options: dark or light.";

pub(super) fn edit_settings(current: &SettingsValues) -> Result<Option<SettingsValues>, AppError> {
    let dialog = Dialog::with_buttons(
        Some("Settings"),
        None::<&Window>,
        DialogFlags::MODAL,
        &[
            ("Cancel", ResponseType::Cancel),
            ("Save", ResponseType::Accept),
        ],
    );
    dialog.set_default_size(560, 520);
    dialog.set_resizable(true);
    dialog.set_keep_above(true);
    dialog.set_default_response(ResponseType::Accept);

    let content = dialog.content_area();
    let container = GtkBox::new(Orientation::Vertical, 12);
    container.set_margin_top(12);
    container.set_margin_bottom(12);
    container.set_margin_start(12);
    container.set_margin_end(12);

    let input_grid = create_section_grid();
    let backend_combo = create_combo(
        &[("portal", "portal"), ("none", "none")],
        input_backend_id(current.input_backend),
        INPUT_BACKEND_TOOLTIP,
    );
    attach_row(
        &input_grid,
        0,
        "Backend",
        INPUT_BACKEND_TOOLTIP,
        &backend_combo,
    );
    let mode_combo = create_combo(
        &[("push-to-talk", "push-to-talk"), ("toggle", "toggle")],
        input_mode_id(current.input_mode),
        INPUT_MODE_TOOLTIP,
    );
    attach_row(&input_grid, 1, "Mode", INPUT_MODE_TOOLTIP, &mode_combo);
    container.pack_start(&wrap_section("Input", &input_grid), false, false, 0);

    let control_grid = create_section_grid();
    let enable_cli = CheckButton::new();
    enable_cli.set_active(current.enable_cli);
    enable_cli.set_tooltip_text(Some(ENABLE_CLI_TOOLTIP));
    attach_row(
        &control_grid,
        0,
        "Enable CLI",
        ENABLE_CLI_TOOLTIP,
        &enable_cli,
    );
    container.pack_start(&wrap_section("Control", &control_grid), false, false, 0);

    let provider_grid = create_section_grid();
    let provider_kind = create_combo(
        &[
            ("groq", "groq"),
            ("openai", "openai"),
            ("openai-compatible", "openai-compatible"),
        ],
        provider_kind_id(current.provider_kind),
        PROVIDER_KIND_TOOLTIP,
    );
    attach_row(
        &provider_grid,
        0,
        "Kind",
        PROVIDER_KIND_TOOLTIP,
        &provider_kind,
    );
    let base_url = create_entry(&current.provider_base_url, PROVIDER_BASE_URL_TOOLTIP, false);
    attach_row(
        &provider_grid,
        1,
        "Base URL",
        PROVIDER_BASE_URL_TOOLTIP,
        &base_url,
    );
    let model = create_entry(&current.provider_model, PROVIDER_MODEL_TOOLTIP, false);
    attach_row(&provider_grid, 2, "Model", PROVIDER_MODEL_TOOLTIP, &model);
    let api_key = create_entry(&current.provider_api_key, PROVIDER_API_KEY_TOOLTIP, true);
    attach_row(
        &provider_grid,
        3,
        "API Key",
        PROVIDER_API_KEY_TOOLTIP,
        &api_key,
    );
    container.pack_start(&wrap_section("Provider", &provider_grid), false, false, 0);

    let paste_grid = create_section_grid();
    let paste_mode = create_combo(
        &[
            ("ctrl-v", "ctrl-v"),
            ("ctrl-shift-v", "ctrl-shift-v"),
            ("shift-insert", "shift-insert"),
        ],
        paste_mode_id(current.paste_mode),
        PASTE_MODE_TOOLTIP,
    );
    attach_row(&paste_grid, 0, "Mode", PASTE_MODE_TOOLTIP, &paste_mode);
    let append_space = CheckButton::new();
    append_space.set_active(current.append_space);
    append_space.set_tooltip_text(Some(APPEND_SPACE_TOOLTIP));
    attach_row(
        &paste_grid,
        1,
        "Append space",
        APPEND_SPACE_TOOLTIP,
        &append_space,
    );
    container.pack_start(&wrap_section("Paste", &paste_grid), false, false, 0);

    let ui_grid = create_section_grid();
    let theme = create_combo(
        &[("dark", "dark"), ("light", "light")],
        ui_theme_id(current.ui_theme),
        UI_THEME_TOOLTIP,
    );
    attach_row(&ui_grid, 0, "Theme", UI_THEME_TOOLTIP, &theme);
    container.pack_start(&wrap_section("UI", &ui_grid), false, false, 0);

    content.pack_start(&container, true, true, 0);
    dialog.show_all();
    dialog.present();

    let response = dialog.run();
    let result = if response == ResponseType::Accept {
        Some(SettingsValues {
            input_backend: parse_input_backend(&selected_id(&backend_combo, "input backend")?)
                .ok_or_else(|| AppError::message("input backend selection is invalid"))?,
            input_mode: parse_input_mode(&selected_id(&mode_combo, "input mode")?)
                .ok_or_else(|| AppError::message("input mode selection is invalid"))?,
            enable_cli: enable_cli.is_active(),
            provider_kind: parse_provider_kind(&selected_id(&provider_kind, "provider kind")?)
                .ok_or_else(|| AppError::message("provider kind selection is invalid"))?,
            provider_base_url: base_url.text().to_string(),
            provider_model: model.text().to_string(),
            provider_api_key: api_key.text().to_string(),
            paste_mode: parse_paste_mode(&selected_id(&paste_mode, "paste mode")?)
                .ok_or_else(|| AppError::message("paste mode selection is invalid"))?,
            append_space: append_space.is_active(),
            ui_theme: parse_ui_theme(&selected_id(&theme, "UI theme")?)
                .ok_or_else(|| AppError::message("UI theme selection is invalid"))?,
        })
    } else {
        None
    };

    dialog.close();
    Ok(result)
}

fn create_section_grid() -> Grid {
    let grid = Grid::new();
    grid.set_row_spacing(8);
    grid.set_column_spacing(12);
    grid
}

fn wrap_section(title: &str, grid: &Grid) -> Frame {
    let frame = Frame::new(Some(title));
    frame.add(grid);
    frame
}

fn create_combo(options: &[(&str, &str)], selected_id: &str, tooltip: &str) -> ComboBoxText {
    let combo = ComboBoxText::new();
    combo.set_hexpand(true);
    combo.set_tooltip_text(Some(tooltip));

    for (id, label) in options {
        combo.append(Some(id), label);
    }

    if !combo.set_active_id(Some(selected_id)) {
        combo.set_active(Some(0));
    }

    combo
}

fn create_entry(text: &str, tooltip: &str, masked: bool) -> Entry {
    let entry = Entry::new();
    entry.set_hexpand(true);
    entry.set_text(text);
    entry.set_visibility(!masked);
    entry.set_tooltip_text(Some(tooltip));
    entry
}

fn attach_row<W: IsA<Widget>>(grid: &Grid, row: i32, label_text: &str, tooltip: &str, widget: &W) {
    let label = Label::new(Some(label_text));
    label.set_xalign(0.0);
    label.set_tooltip_text(Some(tooltip));
    widget.set_tooltip_text(Some(tooltip));

    grid.attach(&label, 0, row, 1, 1);
    grid.attach(widget, 1, row, 1, 1);
}

fn selected_id(combo: &ComboBoxText, label: &str) -> Result<String, AppError> {
    combo
        .active_id()
        .map(|value| value.to_string())
        .ok_or_else(|| AppError::message(format!("{label} must be selected")))
}
