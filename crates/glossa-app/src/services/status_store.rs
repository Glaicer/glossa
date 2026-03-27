use tokio::sync::watch;

use glossa_core::AppStatus;

/// Shared status publisher for the daemon and CLI.
#[derive(Clone, Debug)]
pub struct StatusStore {
    tx: watch::Sender<AppStatus>,
}

impl StatusStore {
    /// Creates a new status store with the given initial status.
    #[must_use]
    pub fn new(initial: AppStatus) -> Self {
        let (tx, _) = watch::channel(initial);
        Self { tx }
    }

    /// Updates the latest status snapshot.
    pub fn update(&self, status: AppStatus) {
        let _ = self.tx.send(status);
    }

    /// Returns the most recent status snapshot.
    #[must_use]
    pub fn snapshot(&self) -> AppStatus {
        self.tx.borrow().clone()
    }

    /// Creates a new watcher for future status updates.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<AppStatus> {
        self.tx.subscribe()
    }
}
