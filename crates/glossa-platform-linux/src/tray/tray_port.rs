use std::{
    env, fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Mutex,
    },
    thread,
    time::Duration,
};

use async_trait::async_trait;
use camino::Utf8Path;
use gtk::{events_pending, init as gtk_init, main_iteration_do};
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
use glossa_core::{AppCommand, CommandOrigin, UiConfig};

const TRAY_THREAD_NAME: &str = "glossa-tray";

/// Best-effort tray port that runs a GTK/AppIndicator tray on Ubuntu GNOME Wayland.
pub struct BestEffortTrayPort {
    enabled: bool,
    ui: UiConfig,
    command_tx: Mutex<Option<mpsc::Sender<TrayCommand>>>,
    started: AtomicBool,
}

impl BestEffortTrayPort {
    #[must_use]
    pub fn new(ui: UiConfig) -> Self {
        Self {
            enabled: ui.tray,
            ui,
            command_tx: Mutex::new(None),
            started: AtomicBool::new(false),
        }
    }

    pub fn bind_command_sender(&self, app_command_tx: UnboundedSender<AppCommand>) {
        if !self.enabled {
            info!("tray is disabled by configuration");
            return;
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
        if let Err(error) = thread::Builder::new()
            .name(TRAY_THREAD_NAME.into())
            .spawn(move || run_tray_thread(ui, app_command_tx, command_rx))
        {
            warn!(error = %error, "failed to spawn tray thread");
            self.started.store(false, Ordering::SeqCst);
            if let Ok(mut guard) = self.command_tx.lock() {
                *guard = None;
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

    async fn show_error(&self, message: &str) -> Result<(), AppError> {
        self.send_command(TrayCommand::show_error(message.to_owned()));
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum TrayCommand {
    SetState(TrayState),
    ShowError(()),
}

impl TrayCommand {
    fn show_error(_message: String) -> Self {
        Self::ShowError(())
    }
}

fn run_tray_thread(
    ui: UiConfig,
    app_command_tx: UnboundedSender<AppCommand>,
    command_rx: mpsc::Receiver<TrayCommand>,
) {
    if let Err(error) = gtk_init() {
        warn!(error = %error, "failed to initialize GTK tray runtime");
        return;
    }

    let runtime = match TrayRuntime::new(&ui) {
        Ok(runtime) => runtime,
        Err(error) => {
            warn!(error = %error, "failed to create tray icon");
            return;
        }
    };

    info!("tray icon initialized");
    loop {
        pump_gtk_events();
        runtime.handle_menu_events(&app_command_tx);

        match command_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(TrayCommand::SetState(state)) => {
                if let Err(error) = runtime.set_state(state) {
                    warn!(error = %error, ?state, "failed to update tray icon");
                }
            }
            Ok(TrayCommand::ShowError(_)) => {
                warn!("tray notifications are not implemented; error is logged only");
            }
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
    exit_id: tray_icon::menu::MenuId,
    icons: TrayIcons,
}

impl TrayRuntime {
    fn new(ui: &UiConfig) -> Result<Self, String> {
        let icons = TrayIcons::load(ui)?;
        let exit_item = MenuItem::new("Exit", true, None);
        let exit_id = exit_item.id().clone();

        let menu = Menu::new();
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
            exit_id,
            icons,
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

    fn handle_menu_events(&self, app_command_tx: &UnboundedSender<AppCommand>) {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id != self.exit_id {
                continue;
            }

            if let Err(error) = app_command_tx.send(AppCommand::Shutdown {
                origin: CommandOrigin::TrayMenu,
            }) {
                warn!(error = %error, "failed to send shutdown command from tray");
            }
        }
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
