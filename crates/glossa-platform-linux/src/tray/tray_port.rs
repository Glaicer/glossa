use std::{
    cell::RefCell,
    env, fs,
    path::PathBuf,
    process::Command,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Mutex,
    },
    thread,
    time::Duration,
};

use async_trait::async_trait;
use camino::Utf8Path;
use gtk::{
    gdk::{self, keys::constants as keyconst},
    glib::{translate::IntoGlib, Propagation},
    prelude::*,
    Box as GtkBox, ButtonsType, Dialog, DialogFlags, Label, MessageDialog, MessageType,
    Orientation, ResponseType, Window, events_pending, init as gtk_init, main_iteration_do,
};
use png::{ColorType, Decoder};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

use glossa_app::{
    ports::{TrayPort, TrayState},
    AppError,
};
use glossa_core::{AppCommand, CommandOrigin, InputBackend, InputConfig, InputMode, UiConfig};

use crate::shortcut_capture::begin_shortcut_capture;
use crate::portal::{portal_shortcut_description, PORTAL_APP_ID, PORTAL_SHORTCUT_ID};

const TRAY_THREAD_NAME: &str = "glossa-tray";

/// Best-effort tray port that runs a GTK/AppIndicator tray on Ubuntu GNOME Wayland.
pub struct BestEffortTrayPort {
    enabled: bool,
    ui: UiConfig,
    shortcut_binding: Option<ShortcutBindingConfig>,
    app_command_tx: std::sync::Arc<Mutex<Option<UnboundedSender<AppCommand>>>>,
    command_tx: Mutex<Option<mpsc::Sender<TrayCommand>>>,
    thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
    started: AtomicBool,
}

impl BestEffortTrayPort {
    #[must_use]
    pub fn new(ui: UiConfig, input: InputConfig) -> Self {
        Self {
            enabled: ui.tray,
            ui,
            shortcut_binding: (input.backend == InputBackend::Portal).then_some(
                ShortcutBindingConfig {
                    current_shortcut: None,
                    mode: input.mode,
                },
            ),
            app_command_tx: std::sync::Arc::new(Mutex::new(None)),
            command_tx: Mutex::new(None),
            thread_handle: Mutex::new(None),
            started: AtomicBool::new(false),
        }
    }

    pub fn bind_command_sender(&self, app_command_tx: UnboundedSender<AppCommand>) {
        if !self.enabled {
            info!("tray is disabled by configuration");
            return;
        }

        match self.app_command_tx.lock() {
            Ok(mut guard) => {
                *guard = Some(app_command_tx);
            }
            Err(_) => {
                warn!("tray app command mutex is poisoned; skipping tray initialization");
                return;
            }
        }

        if self.started.swap(true, Ordering::SeqCst) {
            return;
        }

        if !is_supported_tray_environment() {
            warn!("tray is only enabled on Ubuntu GNOME Wayland; skipping tray initialization");
            self.started.store(false, Ordering::SeqCst);
            return;
        }

        let (command_tx, command_rx) = mpsc::channel();
        match self.command_tx.lock() {
            Ok(mut guard) => {
                *guard = Some(command_tx);
            }
            Err(_) => {
                warn!("tray command mutex is poisoned; skipping tray initialization");
                self.started.store(false, Ordering::SeqCst);
                return;
            }
        }

        let ui = self.ui.clone();
        let shortcut_binding = self.shortcut_binding.clone();
        let app_command_tx = self.app_command_tx.clone();
        let handle = match thread::Builder::new()
            .name(TRAY_THREAD_NAME.into())
            .spawn(move || run_tray_thread(ui, shortcut_binding, app_command_tx, command_rx))
        {
            Ok(handle) => handle,
            Err(error) => {
                warn!(error = %error, "failed to spawn tray thread");
                self.started.store(false, Ordering::SeqCst);
                if let Ok(mut guard) = self.command_tx.lock() {
                    *guard = None;
                }
                return;
            }
        };

        match self.thread_handle.lock() {
            Ok(mut guard) => {
                *guard = Some(handle);
            }
            Err(_) => {
                warn!("tray thread handle mutex is poisoned; shutting tray thread down");
                self.started.store(false, Ordering::SeqCst);
                if let Ok(mut guard) = self.command_tx.lock() {
                    *guard = None;
                }
                let _ = handle.join();
            }
        }
    }

    fn send_command(&self, command: TrayCommand) {
        let sender = match self.command_tx.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => {
                warn!("tray command mutex is poisoned");
                None
            }
        };

        let Some(sender) = sender else {
            return;
        };

        if sender.send(command).is_err() {
            warn!("tray thread is unavailable; dropping tray update");
            if let Ok(mut guard) = self.command_tx.lock() {
                *guard = None;
            }
            self.started.store(false, Ordering::SeqCst);
        }
    }
}

#[async_trait]
impl TrayPort for BestEffortTrayPort {
    async fn set_state(&self, state: TrayState) -> Result<(), AppError> {
        self.send_command(TrayCommand::SetState(state));
        Ok(())
    }

    async fn set_shortcut_description(&self, description: Option<&str>) -> Result<(), AppError> {
        self.send_command(TrayCommand::SetShortcutDescription(
            description.map(ToOwned::to_owned),
        ));
        Ok(())
    }

    async fn show_error(&self, message: &str) -> Result<(), AppError> {
        self.send_command(TrayCommand::show_error(message.to_owned()));
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ShortcutBindingConfig {
    current_shortcut: Option<String>,
    mode: InputMode,
}

#[derive(Clone)]
enum TrayCommand {
    SetState(TrayState),
    SetShortcutDescription(Option<String>),
    ShowError(()),
    Shutdown,
}

impl TrayCommand {
    fn show_error(_message: String) -> Self {
        Self::ShowError(())
    }
}

fn run_tray_thread(
    ui: UiConfig,
    shortcut_binding: Option<ShortcutBindingConfig>,
    app_command_tx: std::sync::Arc<Mutex<Option<UnboundedSender<AppCommand>>>>,
    command_rx: mpsc::Receiver<TrayCommand>,
) {
    if let Err(error) = gtk_init() {
        warn!(error = %error, "failed to initialize GTK tray runtime");
        return;
    }

    let runtime = match TrayRuntime::new(&ui, shortcut_binding, app_command_tx) {
        Ok(runtime) => runtime,
        Err(error) => {
            warn!(error = %error, "failed to create tray icon");
            return;
        }
    };

    info!("tray icon initialized");
    loop {
        pump_gtk_events();
        runtime.handle_menu_events();

        match command_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(TrayCommand::SetState(state)) => {
                if let Err(error) = runtime.set_state(state) {
                    warn!(error = %error, ?state, "failed to update tray icon");
                }
            }
            Ok(TrayCommand::SetShortcutDescription(description)) => {
                runtime.set_shortcut_description(description);
            }
            Ok(TrayCommand::ShowError(_)) => {
                warn!("tray notifications are not implemented; error is logged only");
            }
            Ok(TrayCommand::Shutdown) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn pump_gtk_events() {
    while events_pending() {
        main_iteration_do(false);
    }
}

struct TrayRuntime {
    _tray_icon: TrayIcon,
    change_shortcut_id: Option<tray_icon::menu::MenuId>,
    exit_id: tray_icon::menu::MenuId,
    icons: TrayIcons,
    shortcut_binding: RefCell<Option<ShortcutBindingConfig>>,
    app_command_tx: std::sync::Arc<Mutex<Option<UnboundedSender<AppCommand>>>>,
}

impl TrayRuntime {
    fn new(
        ui: &UiConfig,
        shortcut_binding: Option<ShortcutBindingConfig>,
        app_command_tx: std::sync::Arc<Mutex<Option<UnboundedSender<AppCommand>>>>,
    ) -> Result<Self, String> {
        let icons = TrayIcons::load(ui)?;
        let change_shortcut_item = MenuItem::new("Change shortcut", true, None);
        let change_shortcut_id = shortcut_binding
            .as_ref()
            .map(|_| change_shortcut_item.id().clone());
        let exit_item = MenuItem::new("Exit", true, None);
        let exit_id = exit_item.id().clone();

        let menu = Menu::new();
        if shortcut_binding.is_some() {
            menu.append(&change_shortcut_item)
                .map_err(|error| format!("failed to build tray menu: {error}"))?;
        }
        menu.append(&exit_item)
            .map_err(|error| format!("failed to build tray menu: {error}"))?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icons.idle.clone())
            .with_temp_dir_path(tray_icon_temp_dir()?)
            .build()
            .map_err(|error| format!("failed to create tray icon: {error}"))?;

        Ok(Self {
            _tray_icon: tray_icon,
            change_shortcut_id,
            exit_id,
            icons,
            shortcut_binding: RefCell::new(shortcut_binding),
            app_command_tx,
        })
    }

    fn set_state(&self, state: TrayState) -> Result<(), String> {
        let icon = match state {
            TrayState::Idle => self.icons.idle.clone(),
            TrayState::Recording => self.icons.recording.clone(),
            TrayState::Processing => self.icons.processing.clone(),
        };

        self._tray_icon
            .set_icon(Some(icon))
            .map_err(|error| format!("failed to set tray icon: {error}"))
    }

    fn set_shortcut_description(&self, description: Option<String>) {
        if let Some(binding) = self.shortcut_binding.borrow_mut().as_mut() {
            binding.current_shortcut = description;
        }
    }

    fn handle_menu_events(&self) {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if self
                .change_shortcut_id
                .as_ref()
                .is_some_and(|id| id == &event.id)
            {
                self.handle_change_shortcut();
                continue;
            }

            if event.id != self.exit_id {
                continue;
            }

            if let Err(error) = self.send_app_command(AppCommand::Shutdown {
                origin: CommandOrigin::TrayMenu,
            }) {
                warn!(error = %error, "failed to send shutdown command from tray");
            }
        }
    }

    fn handle_change_shortcut(&self) {
        let Some(binding) = self.shortcut_binding.borrow().clone() else {
            return;
        };

        let captured = match capture_shortcut(&binding) {
            Ok(result) => result,
            Err(error) => {
                warn!(error = %error, "failed to capture shortcut from tray");
                show_message_dialog("Change shortcut", &error, MessageType::Error);
                return;
            }
        };

        let Some(shortcut) = captured else {
            info!("shortcut change cancelled from tray");
            return;
        };

        if let Err(error) = write_shortcut_override(&binding, &shortcut) {
            warn!(error = %error, shortcut = %shortcut, "failed to update GNOME shortcut override");
            show_message_dialog("Change shortcut", &error, MessageType::Error);
            return;
        }

        if let Some(current) = self.shortcut_binding.borrow_mut().as_mut() {
            current.current_shortcut = Some(shortcut.clone());
        }

        info!(shortcut = %shortcut, "updated GNOME shortcut override from tray");
        if let Err(error) = self.send_app_command(AppCommand::Restart {
            origin: CommandOrigin::TrayMenu,
        }) {
            warn!(error = %error, "failed to restart daemon after shortcut change");
            show_message_dialog(
                "Change shortcut",
                "The shortcut was stored, but the daemon could not restart. Restart it manually.",
                MessageType::Warning,
            );
        }
    }

    fn send_app_command(&self, command: AppCommand) -> Result<(), AppError> {
        let sender = match self.app_command_tx.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => {
                return Err(AppError::message("tray app command mutex is poisoned"));
            }
        };

        let Some(sender) = sender else {
            return Err(AppError::message("daemon command sender is not bound"));
        };

        sender
            .send(command)
            .map_err(|_| AppError::message("daemon command channel is closed"))
    }
}

struct TrayIcons {
    idle: Icon,
    recording: Icon,
    processing: Icon,
}

impl TrayIcons {
    fn load(ui: &UiConfig) -> Result<Self, String> {
        let processing_icon_path = ui.processing_icon.as_deref().unwrap_or(&ui.idle_icon);
        Ok(Self {
            idle: load_icon(&ui.idle_icon)?,
            recording: load_icon(&ui.recording_icon)?,
            processing: load_icon(processing_icon_path)?,
        })
    }
}

fn load_icon(path: &Utf8Path) -> Result<Icon, String> {
    let file = fs::File::open(path)
        .map_err(|error| format!("failed to open tray icon {path}: {error}"))?;
    let decoder = Decoder::new(file);

    let mut reader = decoder
        .read_info()
        .map_err(|error| format!("failed to decode tray icon {path}: {error}"))?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|error| format!("failed to read tray icon {path}: {error}"))?;
    let rgba = normalize_rgba(&buffer[..info.buffer_size()], info.color_type)
        .map_err(|error| format!("failed to normalize tray icon {path}: {error}"))?;

    Icon::from_rgba(rgba, info.width, info.height)
        .map_err(|error| format!("failed to convert tray icon {path}: {error}"))
}

fn normalize_rgba(bytes: &[u8], color_type: ColorType) -> Result<Vec<u8>, &'static str> {
    let rgba = match color_type {
        ColorType::Rgba => bytes.to_vec(),
        ColorType::Rgb => bytes
            .chunks_exact(3)
            .flat_map(|chunk| [chunk[0], chunk[1], chunk[2], u8::MAX])
            .collect(),
        ColorType::Grayscale => bytes
            .iter()
            .flat_map(|value| [*value, *value, *value, u8::MAX])
            .collect(),
        ColorType::GrayscaleAlpha => bytes
            .chunks_exact(2)
            .flat_map(|chunk| [chunk[0], chunk[0], chunk[0], chunk[1]])
            .collect(),
        ColorType::Indexed => return Err("indexed PNG tray icons are unsupported"),
    };

    Ok(rgba)
}

fn tray_icon_temp_dir() -> Result<PathBuf, String> {
    let path = if let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("glossa").join("tray-icon")
    } else {
        env::temp_dir().join("glossa").join("tray-icon")
    };

    fs::create_dir_all(&path).map_err(|error| {
        format!(
            "failed to prepare tray temp dir {}: {error}",
            path.display()
        )
    })?;
    Ok(path)
}

fn capture_shortcut(binding: &ShortcutBindingConfig) -> Result<Option<String>, String> {
    let _capture_guard = begin_shortcut_capture();
    let dialog = Dialog::with_buttons(
        Some("Change shortcut"),
        None::<&Window>,
        DialogFlags::MODAL,
        &[("Cancel", ResponseType::Cancel)],
    );
    dialog.set_resizable(false);
    dialog.set_keep_above(true);

    let content = dialog.content_area();
    let container = GtkBox::new(Orientation::Vertical, 8);
    container.set_margin_top(12);
    container.set_margin_bottom(12);
    container.set_margin_start(12);
    container.set_margin_end(12);

    let title = Label::new(Some("Press the new shortcut"));
    title.set_xalign(0.0);
    let detail = Label::new(Some(
        "Glossa will write the binding into GNOME's shortcut storage and restart.\nPress Esc to cancel.",
    ));
    detail.set_xalign(0.0);
    detail.set_line_wrap(true);
    let current_text = binding.current_shortcut.as_deref().map_or(
        "Current shortcut is managed outside config.toml.".to_owned(),
        |shortcut| format!("Current stored shortcut: {shortcut}"),
    );
    let current = Label::new(Some(&current_text));
    current.set_xalign(0.0);
    let preview = Label::new(Some("Waiting for input..."));
    preview.set_xalign(0.0);

    container.pack_start(&title, false, false, 0);
    container.pack_start(&detail, false, false, 0);
    container.pack_start(&current, false, false, 0);
    container.pack_start(&preview, false, false, 0);
    content.pack_start(&container, true, true, 0);

    let captured = Rc::new(RefCell::new(None::<String>));
    let captured_ref = Rc::clone(&captured);
    let preview_ref = preview.clone();
    dialog.connect_key_press_event(move |dialog, event| {
        if event.keyval() == keyconst::Escape {
            dialog.response(ResponseType::Cancel);
            return Propagation::Stop;
        }

        if is_modifier_key(event.keyval()) {
            preview_ref.set_text("Keep holding modifiers and press a non-modifier key.");
            return Propagation::Stop;
        }

        let shortcut = shortcut_from_event(event);
        if shortcut.is_empty() {
            preview_ref.set_text("That key combination is not supported.");
            return Propagation::Stop;
        }

        preview_ref.set_text(&format!("Selected: {shortcut}"));
        *captured_ref.borrow_mut() = Some(shortcut);
        dialog.response(ResponseType::Accept);
        Propagation::Stop
    });

    dialog.show_all();
    dialog.present();
    let response = dialog.run();
    dialog.close();

    if response == ResponseType::Accept {
        Ok(captured.borrow().clone())
    } else {
        Ok(None)
    }
}

fn shortcut_from_event(event: &gdk::EventKey) -> String {
    let modifiers = event.state() & gtk::accelerator_get_default_mod_mask();
    gtk::accelerator_name(event.keyval().into_glib(), modifiers)
        .map(|shortcut| normalize_captured_shortcut(shortcut.as_str()))
        .unwrap_or_default()
}

fn normalize_captured_shortcut(shortcut: &str) -> String {
    shortcut
        .replace("<Primary>", "<Ctrl>")
        .replace("<Control>", "<Ctrl>")
        .replace("<Mod1>", "<Alt>")
}

fn is_modifier_key(keyval: gdk::keys::Key) -> bool {
    matches!(
        keyval,
        keyconst::Shift_L
            | keyconst::Shift_R
            | keyconst::Control_L
            | keyconst::Control_R
            | keyconst::Alt_L
            | keyconst::Alt_R
            | keyconst::Meta_L
            | keyconst::Meta_R
            | keyconst::Super_L
            | keyconst::Super_R
            | keyconst::Hyper_L
            | keyconst::Hyper_R
    )
}

fn write_shortcut_override(binding: &ShortcutBindingConfig, shortcut: &str) -> Result<(), String> {
    let payload = shortcut_override_value(binding, shortcut);
    let output = Command::new("dconf")
        .arg("write")
        .arg(shortcut_override_path())
        .arg(payload)
        .output()
        .map_err(|error| format!("failed to launch dconf: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if stderr.is_empty() {
        Err(format!(
            "dconf write failed with status {}",
            output.status
        ))
    } else {
        Err(format!("dconf write failed: {stderr}"))
    }
}

fn shortcut_override_path() -> String {
    format!("/org/gnome/settings-daemon/global-shortcuts/{PORTAL_APP_ID}/shortcuts")
}

fn shortcut_override_value(binding: &ShortcutBindingConfig, shortcut: &str) -> String {
    format!(
        "[('{}', {{'shortcuts': <['{}']>, 'description': <'{}'>}})]",
        PORTAL_SHORTCUT_ID,
        escape_dconf_string(shortcut),
        escape_dconf_string(portal_shortcut_description(binding.mode)),
    )
}

fn escape_dconf_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn show_message_dialog(title: &str, message: &str, message_type: MessageType) {
    let dialog = MessageDialog::new(
        None::<&Window>,
        DialogFlags::MODAL,
        message_type,
        ButtonsType::Close,
        message,
    );
    dialog.set_title(title);
    let _ = dialog.run();
    dialog.close();
}

impl Drop for BestEffortTrayPort {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.command_tx.lock() {
            if let Some(sender) = guard.take() {
                let _ = sender.send(TrayCommand::Shutdown);
            }
        }

        if let Ok(mut guard) = self.thread_handle.lock() {
            if let Some(handle) = guard.take() {
                if handle.thread().id() != thread::current().id() {
                    let _ = handle.join();
                }
            }
        }
    }
}

fn is_supported_tray_environment() -> bool {
    is_wayland_session() && is_gnome_session() && is_ubuntu()
}

fn is_wayland_session() -> bool {
    matches!(
        env::var("XDG_SESSION_TYPE"),
        Ok(session_type) if session_type.eq_ignore_ascii_case("wayland")
    ) || env::var_os("WAYLAND_DISPLAY").is_some()
}

fn is_gnome_session() -> bool {
    let desktop = env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| env::var("DESKTOP_SESSION"))
        .unwrap_or_default();
    desktop.to_ascii_uppercase().contains("GNOME")
}

fn is_ubuntu() -> bool {
    let Ok(os_release) = fs::read_to_string("/etc/os-release") else {
        return false;
    };

    os_release.lines().any(|line| {
        let value = line
            .strip_prefix("ID=")
            .or_else(|| line.strip_prefix("ID_LIKE="));

        value.is_some_and(|value| {
            value
                .trim_matches('"')
                .split_ascii_whitespace()
                .any(|token| token.eq_ignore_ascii_case("ubuntu"))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{normalize_captured_shortcut, shortcut_override_value, ShortcutBindingConfig};
    use glossa_core::InputMode;

    #[test]
    fn captured_shortcut_should_use_ctrl_alias() {
        assert_eq!(
            normalize_captured_shortcut("<Primary><Shift>r"),
            "<Ctrl><Shift>r"
        );
    }

    #[test]
    fn shortcut_override_value_should_include_shortcut_and_description() {
        let value = shortcut_override_value(
            &ShortcutBindingConfig {
                current_shortcut: Some("<Ctrl><Alt>space".into()),
                mode: InputMode::Toggle,
            },
            "<Ctrl><Shift>r",
        );

        assert!(value.contains("'main'"));
        assert!(value.contains("<['<Ctrl><Shift>r']>"));
        assert!(value.contains("Toggle Glossa recording"));
    }
}
