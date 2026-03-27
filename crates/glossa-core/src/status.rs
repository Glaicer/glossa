use serde::{Deserialize, Serialize};

use crate::{AppStateKind, ProviderKind};

/// Lightweight status snapshot exposed to the CLI and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppStatus {
    pub state: AppStateKind,
    pub provider: ProviderKind,
    pub tray_available: bool,
    pub portal_available: bool,
}
