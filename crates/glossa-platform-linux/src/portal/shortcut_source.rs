use std::future::pending;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::warn;

use glossa_app::{ports::CommandSource, AppError};
use glossa_core::{AppCommand, InputConfig};

/// Placeholder portal command source with pure signal mapping and graceful degradation.
#[derive(Debug, Clone)]
pub struct PortalShortcutSource {
    config: InputConfig,
}

impl PortalShortcutSource {
    #[must_use]
    pub fn new(config: InputConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl CommandSource for PortalShortcutSource {
    fn name(&self) -> &'static str {
        "portal-shortcut"
    }

    async fn run(self: Box<Self>, _tx: mpsc::UnboundedSender<AppCommand>) -> Result<(), AppError> {
        warn!(
            shortcut = %self.config.shortcut,
            mode = ?self.config.mode,
            "portal integration is currently a best-effort placeholder and will stay idle"
        );
        pending::<()>().await;
        Ok(())
    }
}
