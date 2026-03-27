use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::AppError;
use glossa_core::AppCommand;

/// External source that feeds commands into the daemon.
#[async_trait]
pub trait CommandSource: Send + Sync {
    fn name(&self) -> &'static str;

    async fn run(self: Box<Self>, tx: mpsc::UnboundedSender<AppCommand>) -> Result<(), AppError>;
}
