use serde::{Deserialize, Serialize};

use glossa_core::AppStatus;

/// Supported IPC requests sent by control clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum IpcRequest {
    Toggle,
    Stream,
    Status,
    Shutdown,
}

/// IPC responses returned by the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum IpcResponse {
    Ok,
    Status { status: AppStatus },
    Error { message: String },
}
