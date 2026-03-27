use async_trait::async_trait;
use tracing::{info, warn};

use glossa_app::{
    ports::{TrayPort, TrayState},
    AppError,
};

/// Best-effort tray port that degrades to logging when the environment lacks tray support.
#[derive(Debug, Clone)]
pub struct BestEffortTrayPort {
    enabled: bool,
}

impl BestEffortTrayPort {
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

#[async_trait]
impl TrayPort for BestEffortTrayPort {
    async fn set_state(&self, state: TrayState) -> Result<(), AppError> {
        if self.enabled {
            info!(?state, "tray state updated");
        } else {
            warn!(
                ?state,
                "tray update skipped because tray support is disabled"
            );
        }
        Ok(())
    }

    async fn show_error(&self, message: &str) -> Result<(), AppError> {
        if self.enabled {
            warn!(message, "tray error notification");
        }
        Ok(())
    }
}
