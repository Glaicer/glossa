use async_trait::async_trait;
use tokio::{io::AsyncWriteExt, process::Command};

use glossa_app::{ports::ClipboardWriter, AppError};

/// Clipboard writer backed by the `wl-copy` binary.
#[derive(Debug, Clone)]
pub struct WlCopyClipboard {
    command: String,
}

impl WlCopyClipboard {
    #[must_use]
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }
}

#[async_trait]
impl ClipboardWriter for WlCopyClipboard {
    async fn set_text(&self, text: &str) -> Result<(), AppError> {
        let mut child = Command::new(&self.command)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|error| AppError::io("failed to spawn wl-copy", error))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(text.as_bytes())
                .await
                .map_err(|error| AppError::io("failed to write clipboard text", error))?;
        }

        let status = child
            .wait()
            .await
            .map_err(|error| AppError::io("failed to wait for wl-copy", error))?;
        if status.success() {
            Ok(())
        } else {
            Err(AppError::message(format!(
                "wl-copy exited with status {status}"
            )))
        }
    }
}
