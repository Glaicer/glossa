use std::env;

use camino::Utf8PathBuf;

use glossa_app::AppError;
use glossa_core::AppConfig;

use super::{DoctorFinding, DoctorLevel, DoctorReport};

/// Environment diagnostics for the Glossa runtime.
#[derive(Debug, Clone)]
pub struct Doctor;

impl Doctor {
    pub async fn run(config: &AppConfig) -> Result<DoctorReport, AppError> {
        let findings = vec![
            check_wayland(),
            check_gnome(),
            check_session_bus(),
            check_portal(config),
            check_binary("wl-copy"),
            check_binary("dotool"),
            check_tray(config),
            check_socket(config),
            check_config(config),
            check_api_key(config),
        ];

        Ok(DoctorReport { findings })
    }
}

fn check_wayland() -> DoctorFinding {
    if env::var_os("WAYLAND_DISPLAY").is_some() {
        ok("Wayland session", "WAYLAND_DISPLAY is set")
    } else {
        fail("Wayland session", "WAYLAND_DISPLAY is not set")
    }
}

fn check_gnome() -> DoctorFinding {
    let desktop = env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| env::var("DESKTOP_SESSION"))
        .unwrap_or_default();
    if desktop.to_ascii_uppercase().contains("GNOME") {
        ok("GNOME desktop", format!("detected {desktop}"))
    } else {
        warn(
            "GNOME desktop",
            if desktop.is_empty() {
                "GNOME was not detected".into()
            } else {
                format!("desktop session is {desktop}")
            },
        )
    }
}

fn check_session_bus() -> DoctorFinding {
    if env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some() {
        ok("Session bus", "DBUS_SESSION_BUS_ADDRESS is set")
    } else {
        fail("Session bus", "DBUS_SESSION_BUS_ADDRESS is not set")
    }
}

fn check_portal(config: &AppConfig) -> DoctorFinding {
    if config.input.backend == glossa_core::InputBackend::None {
        warn(
            "Portal GlobalShortcuts",
            "portal backend is disabled by configuration",
        )
    } else if env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some() {
        warn(
            "Portal GlobalShortcuts",
            "session bus is available, but portal registration is not proactively probed by doctor",
        )
    } else {
        fail(
            "Portal GlobalShortcuts",
            "session bus is unavailable, so portal integration will not work",
        )
    }
}

fn check_binary(binary: &'static str) -> DoctorFinding {
    match find_binary(binary) {
        Some(path) => ok(binary, format!("found at {}", path.display())),
        None => fail(binary, format!("{binary} was not found on PATH")),
    }
}

fn check_tray(config: &AppConfig) -> DoctorFinding {
    if !config.ui.tray {
        warn("Tray backend", "tray is disabled by configuration")
    } else {
        warn(
            "Tray backend",
            "GNOME AppIndicator support cannot be auto-confirmed; the daemon will degrade gracefully",
        )
    }
}

fn check_socket(config: &AppConfig) -> DoctorFinding {
    match config.control.socket_path.resolve() {
        Ok(path) => {
            let fallback = Utf8PathBuf::from(".");
            let parent = path.parent().unwrap_or(fallback.as_path());
            if parent.exists() {
                ok(
                    "Runtime socket",
                    format!("socket directory exists: {parent}"),
                )
            } else {
                warn(
                    "Runtime socket",
                    format!("socket directory will be created on demand: {parent}"),
                )
            }
        }
        Err(error) => fail("Runtime socket", error.to_string()),
    }
}

fn check_config(config: &AppConfig) -> DoctorFinding {
    match config.validate() {
        Ok(()) => ok("Config", "configuration is valid"),
        Err(error) => fail("Config", error.to_string()),
    }
}

fn check_api_key(config: &AppConfig) -> DoctorFinding {
    match config.resolve_api_key() {
        Ok(_) => ok(
            "API key",
            format!("resolved from {}", config.provider.api_key.describe()),
        ),
        Err(error) => fail("API key", error.to_string()),
    }
}

fn ok(name: &'static str, detail: impl Into<String>) -> DoctorFinding {
    DoctorFinding {
        level: DoctorLevel::Ok,
        name,
        detail: detail.into(),
    }
}

fn warn(name: &'static str, detail: impl Into<String>) -> DoctorFinding {
    DoctorFinding {
        level: DoctorLevel::Warn,
        name,
        detail: detail.into(),
    }
}

fn fail(name: &'static str, detail: impl Into<String>) -> DoctorFinding {
    DoctorFinding {
        level: DoctorLevel::Fail,
        name,
        detail: detail.into(),
    }
}

fn find_binary(binary: &str) -> Option<std::path::PathBuf> {
    let path_var = env::var_os("PATH")?;
    env::split_paths(&path_var)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn doctor_report_should_render_levels() {
        let report = Doctor::run(&AppConfig::default())
            .await
            .expect("doctor should produce a report");
        let rendered = report.to_string();
        assert!(
            rendered.contains("["),
            "report should render severity labels"
        );
    }
}
