use std::time::Duration;
use std::{
    collections::HashMap,
    env, fs,
    path::PathBuf,
    sync::Arc,
    sync::atomic::{AtomicU64, Ordering},
};

use ashpd::{
    desktop::global_shortcuts::NewShortcut,
    zbus,
    zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Type, Value},
    WindowIdentifier,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use glossa_app::{
    ports::{CommandSource, TrayPort},
    AppError,
};
use glossa_core::{AppCommand, InputConfig, InputMode};

use crate::shortcut_capture::is_shortcut_capture_active;

use super::{map_portal_signal_to_command, PortalSignal};

const DESKTOP_DESTINATION: &str = "org.freedesktop.portal.Desktop";
const DESKTOP_PATH: &str = "/org/freedesktop/portal/desktop";
const GLOBAL_SHORTCUTS_INTERFACE: &str = "org.freedesktop.portal.GlobalShortcuts";
const HOST_REGISTRY_INTERFACE: &str = "org.freedesktop.host.portal.Registry";
const REQUEST_INTERFACE: &str = "org.freedesktop.portal.Request";
pub(crate) const PORTAL_APP_ID: &str = "dev.glaicer.glossa";
pub(crate) const PORTAL_SHORTCUT_ID: &str = "main";
const RETRY_DELAY: Duration = Duration::from_secs(15);
static REQUEST_TOKEN_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
struct BoundShortcut {
    id: String,
    trigger_description: String,
}

impl BoundShortcut {
    fn id(&self) -> &str {
        &self.id
    }

    fn trigger_description(&self) -> &str {
        &self.trigger_description
    }
}

/// Global-shortcut portal source for Wayland sessions.
#[derive(Clone)]
pub struct PortalShortcutSource {
    config: InputConfig,
    tray: Option<Arc<dyn TrayPort>>,
}

impl PortalShortcutSource {
    #[must_use]
    pub fn new(config: InputConfig, tray: Option<Arc<dyn TrayPort>>) -> Self {
        Self { config, tray }
    }
}

#[async_trait]
impl CommandSource for PortalShortcutSource {
    fn name(&self) -> &'static str {
        "portal-shortcut"
    }

    async fn run(self: Box<Self>, tx: mpsc::UnboundedSender<AppCommand>) -> Result<(), AppError> {
        loop {
            if let Err(error) = self.run_session(&tx).await {
                warn!(
                    error = %error,
                    mode = ?self.config.mode,
                    retry_delay_sec = RETRY_DELAY.as_secs(),
                    "portal shortcut integration is unavailable; cli control remains active"
                );
                sleep(RETRY_DELAY).await;
            }
        }
    }
}

impl PortalShortcutSource {
    async fn run_session(&self, tx: &mpsc::UnboundedSender<AppCommand>) -> Result<(), AppError> {
        let connection = zbus::Connection::session().await.map_err(|error| {
            AppError::message(format!("failed to connect to session bus: {error}"))
        })?;
        let proxy = zbus::Proxy::new(
            &connection,
            DESKTOP_DESTINATION,
            DESKTOP_PATH,
            GLOBAL_SHORTCUTS_INTERFACE,
        )
        .await
        .map_err(|error| {
            AppError::message(format!(
                "failed to connect to GlobalShortcuts portal: {error}"
            ))
        })?;
        register_host_app(&connection).await?;
        let session = create_session(&connection, &proxy).await?;
        ensure_shortcut_binding(
            &connection,
            &proxy,
            &session,
            self.config.mode,
            self.tray.as_deref(),
        )
        .await?;

        let mut activated = proxy.receive_signal("Activated").await.map_err(|error| {
            AppError::message(format!(
                "failed to subscribe to portal activation events: {error}"
            ))
        })?;
        let mut deactivated = proxy.receive_signal("Deactivated").await.map_err(|error| {
            AppError::message(format!(
                "failed to subscribe to portal deactivation events: {error}"
            ))
        })?;

        loop {
            tokio::select! {
                signal = activated.next() => match signal {
                    Some(signal) => {
                        let (signal_session, shortcut_id, _, _): (OwnedObjectPath, String, u64, HashMap<String, OwnedValue>) =
                            signal.body().deserialize().map_err(|error| {
                                AppError::message(format!("failed to decode portal activation event: {error}"))
                            })?;
                        self.handle_signal(&session, &signal_session.as_ref(), &shortcut_id, PortalSignal::Activated, tx)?;
                    }
                    None => return Err(AppError::message("portal activation stream ended unexpectedly")),
                },
                signal = deactivated.next() => match signal {
                    Some(signal) => {
                        let (signal_session, shortcut_id, _, _): (OwnedObjectPath, String, u64, HashMap<String, OwnedValue>) =
                            signal.body().deserialize().map_err(|error| {
                                AppError::message(format!("failed to decode portal deactivation event: {error}"))
                            })?;
                        self.handle_signal(&session, &signal_session.as_ref(), &shortcut_id, PortalSignal::Deactivated, tx)?;
                    }
                    None => return Err(AppError::message("portal deactivation stream ended unexpectedly")),
                },
            }
        }
    }

    fn handle_signal(
        &self,
        session: &OwnedObjectPath,
        signal_session: &ObjectPath<'_>,
        shortcut_id: &str,
        signal: PortalSignal,
        tx: &mpsc::UnboundedSender<AppCommand>,
    ) -> Result<(), AppError> {
        if session.as_str() != signal_session.as_str() {
            debug!(
                session_handle = %signal_session,
                expected_session_handle = %session.as_str(),
                signal = ?signal,
                "ignoring portal signal for a different session"
            );
            return Ok(());
        }

        if shortcut_id != PORTAL_SHORTCUT_ID {
            debug!(
                shortcut_id,
                signal = ?signal,
                "ignoring portal signal for an unknown shortcut id"
            );
            return Ok(());
        }

        if is_shortcut_capture_active() {
            debug!(
                shortcut_id,
                signal = ?signal,
                "ignoring portal signal while tray shortcut capture is active"
            );
            return Ok(());
        }

        let Some(command) = map_portal_signal_to_command(self.config.mode, signal) else {
            debug!(shortcut_id, signal = ?signal, "portal signal maps to no command");
            return Ok(());
        };

        debug!(shortcut_id, signal = ?signal, command = ?command, "forwarding portal command");
        tx.send(command)
            .map_err(|_| AppError::message("daemon command channel is closed"))?;
        Ok(())
    }
}

pub(crate) fn portal_shortcut_description(mode: InputMode) -> &'static str {
    match mode {
        InputMode::Toggle => "Toggle Glossa recording",
        InputMode::PushToTalk => "Hold to record with Glossa",
    }
}

async fn create_session(
    connection: &zbus::Connection,
    proxy: &zbus::Proxy<'_>,
) -> Result<OwnedObjectPath, AppError> {
    let handle_token = next_token("request");
    let session_token = next_token("session");
    let session_handle = portal_object_path(connection, "/org/freedesktop/portal/desktop/session", &session_token)?;

    let options = HashMap::from([
        ("handle_token", Value::from(handle_token.as_str())),
        ("session_handle_token", Value::from(session_token.as_str())),
    ]);

    let _: HashMap<String, OwnedValue> = request_response(
        connection,
        proxy,
        "CreateSession",
        &handle_token,
        &options,
    )
    .await?;

    Ok(session_handle)
}

async fn bind_shortcuts(
    connection: &zbus::Connection,
    proxy: &zbus::Proxy<'_>,
    session: &OwnedObjectPath,
    shortcut: &NewShortcut,
) -> Result<Vec<BoundShortcut>, AppError> {
    let handle_token = next_token("request");
    let options = HashMap::from([("handle_token", Value::from(handle_token.as_str()))]);
    let shortcuts = vec![shortcut.clone()];
    let parent_window = WindowIdentifier::default();

    let results: HashMap<String, OwnedValue> = request_response(
        connection,
        proxy,
        "BindShortcuts",
        &handle_token,
        &(session, shortcuts.as_slice(), &parent_window, &options),
    )
    .await?;

    extract_shortcuts(results)
}

async fn list_shortcuts(
    connection: &zbus::Connection,
    proxy: &zbus::Proxy<'_>,
    session: &OwnedObjectPath,
) -> Result<Vec<BoundShortcut>, AppError> {
    let handle_token = next_token("request");
    let options = HashMap::from([("handle_token", Value::from(handle_token.as_str()))]);
    let results: HashMap<String, OwnedValue> = request_response(
        connection,
        proxy,
        "ListShortcuts",
        &handle_token,
        &(session, &options),
    )
    .await?;
    extract_shortcuts(results)
}

async fn ensure_shortcut_binding(
    connection: &zbus::Connection,
    proxy: &zbus::Proxy<'_>,
    session: &OwnedObjectPath,
    mode: InputMode,
    tray: Option<&dyn TrayPort>,
) -> Result<(), AppError> {
    let shortcut = NewShortcut::new(PORTAL_SHORTCUT_ID, portal_shortcut_description(mode));
    let existing = list_shortcuts(connection, proxy, session).await?;
    let effective = if existing
        .iter()
        .any(|shortcut| shortcut.id() == PORTAL_SHORTCUT_ID)
    {
        existing
    } else {
        bind_shortcuts(connection, proxy, session, &shortcut).await?
    };

    if let Some(bound) = effective
        .iter()
        .find(|shortcut| shortcut.id() == PORTAL_SHORTCUT_ID)
    {
        if let Some(tray) = tray {
            let _ = tray
                .set_shortcut_description(Some(bound.trigger_description()))
                .await;
        }

        info!(
            shortcut_id = PORTAL_SHORTCUT_ID,
            effective_trigger = bound.trigger_description(),
            session_handle = %session.as_str(),
            mode = ?mode,
            "portal shortcut is active"
        );
    } else {
        if let Some(tray) = tray {
            let _ = tray.set_shortcut_description(None).await;
        }

        warn!(
            shortcut_id = PORTAL_SHORTCUT_ID,
            session_handle = %session.as_str(),
            "portal session is active but no shortcut with the configured id is bound"
        );
    }

    Ok(())
}

async fn request_response<B, R>(
    connection: &zbus::Connection,
    proxy: &zbus::Proxy<'_>,
    method_name: &'static str,
    handle_token: &str,
    body: &B,
) -> Result<R, AppError>
where
    B: serde::ser::Serialize + Type + std::fmt::Debug,
    R: for<'de> serde::Deserialize<'de> + Type,
{
    let request_path =
        portal_object_path(connection, "/org/freedesktop/portal/desktop/request", handle_token)?;
    let request_proxy = zbus::Proxy::new(
        connection,
        DESKTOP_DESTINATION,
        request_path.as_str(),
        REQUEST_INTERFACE,
    )
    .await
    .map_err(|error| {
        AppError::message(format!(
            "failed to connect to portal request object for {method_name}: {error}"
        ))
    })?;
    let mut response_stream = request_proxy.receive_signal("Response").await.map_err(|error| {
        AppError::message(format!(
            "failed to subscribe to portal {method_name} response: {error}"
        ))
    })?;

    proxy.call_method(method_name, body).await.map_err(|error| {
        AppError::message(format!("failed to call portal {method_name}: {error}"))
    })?;

    let message = response_stream.next().await.ok_or_else(|| {
        AppError::message(format!("portal {method_name} request ended without a response"))
    })?;
    let (response_code, results): (u32, R) =
        message.body().deserialize().map_err(|error| {
            AppError::message(format!(
                "failed to decode portal {method_name} response: {error}"
            ))
        })?;

    match response_code {
        0 => Ok(results),
        1 => Err(AppError::message(format!(
            "portal {method_name} request was cancelled"
        ))),
        2 => Err(AppError::message(format!(
            "portal {method_name} request failed"
        ))),
        other => Err(AppError::message(format!(
            "portal {method_name} returned unknown response code {other}"
        ))),
    }
}

fn portal_object_path(
    connection: &zbus::Connection,
    prefix: &str,
    token: &str,
) -> Result<OwnedObjectPath, AppError> {
    let unique_name = connection
        .unique_name()
        .ok_or_else(|| AppError::message("session bus connection has no unique name"))?;
    let unique_identifier = unique_name.trim_start_matches(':').replace('.', "_");
    let path = format!("{prefix}/{unique_identifier}/{token}");
    OwnedObjectPath::try_from(path).map_err(|error| {
        AppError::message(format!("failed to construct portal object path: {error}"))
    })
}

fn next_token(kind: &str) -> String {
    let value = REQUEST_TOKEN_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("glossa_{kind}_{value}")
}

fn extract_shortcuts(results: HashMap<String, OwnedValue>) -> Result<Vec<BoundShortcut>, AppError> {
    let Some(shortcuts) = results.get("shortcuts") else {
        return Ok(Vec::new());
    };

    let raw_shortcuts: Vec<(String, HashMap<String, OwnedValue>)> = shortcuts
        .try_clone()
        .and_then(TryInto::try_into)
        .map_err(|error| AppError::message(format!("failed to decode portal shortcut list: {error}")))?;

    raw_shortcuts
        .into_iter()
        .map(|(id, info)| {
            let trigger_description = info
                .get("trigger_description")
                .ok_or_else(|| AppError::message("portal shortcut is missing trigger_description"))?
                .try_clone()
                .and_then(TryInto::try_into)
                .map_err(|error| AppError::message(format!("failed to decode portal trigger description: {error}")))?;

            Ok(BoundShortcut {
                id,
                trigger_description,
            })
        })
        .collect()
}

async fn register_host_app(connection: &zbus::Connection) -> Result<(), AppError> {
    let desktop_file = ensure_desktop_entry()?;
    let registry = zbus::Proxy::new(
        connection,
        DESKTOP_DESTINATION,
        DESKTOP_PATH,
        HOST_REGISTRY_INTERFACE,
    )
    .await
    .map_err(|error| AppError::message(format!("failed to connect to host portal registry: {error}")))?;

    let options = HashMap::<String, Value<'_>>::new();
    registry
        .call_method("Register", &(PORTAL_APP_ID, options))
        .await
        .map_err(|error| AppError::message(format!("failed to register portal app id {PORTAL_APP_ID}: {error}")))?;

    info!(app_id = PORTAL_APP_ID, desktop_file = %desktop_file.display(), "registered host app identity for portals");
    Ok(())
}

fn ensure_desktop_entry() -> Result<PathBuf, AppError> {
    let applications_dir = applications_dir()?;
    fs::create_dir_all(&applications_dir)
        .map_err(|error| AppError::io("failed to create applications directory", error))?;

    let desktop_file = applications_dir.join(format!("{PORTAL_APP_ID}.desktop"));
    let desired = desktop_entry_contents()?;
    let needs_write = match fs::read_to_string(&desktop_file) {
        Ok(existing) => existing != desired,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(error) => return Err(AppError::io("failed to read desktop entry", error)),
    };

    if needs_write {
        fs::write(&desktop_file, desired)
            .map_err(|error| AppError::io("failed to write desktop entry", error))?;
    }

    Ok(desktop_file)
}

fn applications_dir() -> Result<PathBuf, AppError> {
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path).join("applications"));
    }

    let home = env::var_os("HOME")
        .ok_or_else(|| AppError::message("HOME is not set; cannot place desktop entry"))?;
    Ok(PathBuf::from(home).join(".local/share/applications"))
}

fn desktop_entry_contents() -> Result<String, AppError> {
    let current_exe = env::current_exe()
        .map_err(|error| AppError::io("failed to resolve current executable path", error))?;
    let exec = format!(
        "{} daemon",
        escape_desktop_exec_arg(&current_exe.to_string_lossy())
    );

    Ok(format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName=Glossa\nComment=Headless speech-to-text daemon\nExec={exec}\nTerminal=false\nNoDisplay=true\nCategories=Utility;\n"
    ))
}

fn escape_desktop_exec_arg(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            ' ' => escaped.push_str("\\s"),
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push_str("\\t"),
            '\r' => escaped.push_str("\\r"),
            '\\' => escaped.push_str("\\\\"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::escape_desktop_exec_arg;

    #[test]
    fn desktop_exec_args_should_escape_spaces() {
        assert_eq!(
            escape_desktop_exec_arg("/tmp/glossa debug"),
            "/tmp/glossa\\sdebug"
        );
    }
}
