use tokio::sync::mpsc;

use glossa_core::{AppCommand, AppStatus};

use crate::{services::status_store::StatusStore, AppError};

/// Handle exposed to IPC and long-running command sources.
#[derive(Clone, Debug)]
pub struct AppHandle {
    command_tx: mpsc::UnboundedSender<AppCommand>,
    status_store: StatusStore,
}

impl AppHandle {
    /// Creates a new application handle from the actor channels.
    #[must_use]
    pub fn new(command_tx: mpsc::UnboundedSender<AppCommand>, status_store: StatusStore) -> Self {
        Self {
            command_tx,
            status_store,
        }
    }

    /// Sends a command to the daemon actor.
    pub fn send(&self, command: AppCommand) -> Result<(), AppError> {
        self.command_tx
            .send(command)
            .map_err(|_| AppError::message("daemon command channel is closed"))
    }

    /// Returns the latest status snapshot.
    #[must_use]
    pub fn status(&self) -> AppStatus {
        self.status_store.snapshot()
    }

    /// Subscribes to future status updates.
    #[must_use]
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<AppStatus> {
        self.status_store.subscribe()
    }

    /// Clones the raw command sender for command sources.
    #[must_use]
    pub fn command_sender(&self) -> mpsc::UnboundedSender<AppCommand> {
        self.command_tx.clone()
    }
}
