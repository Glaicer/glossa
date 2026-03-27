use anyhow::{anyhow, Context};

use glossa_platform_linux::ipc::{IpcClient, IpcRequest, IpcResponse};

use crate::bootstrap::resolve_socket_path;

pub async fn run(config_path: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    let socket_path = resolve_socket_path(config_path).await?;
    let client = IpcClient::new(socket_path);
    let response = client.request(IpcRequest::Status).await?;
    match response {
        IpcResponse::Status { status } => {
            println!(
                "state={:?} provider={:?} tray_available={} portal_available={}",
                status.state, status.provider, status.tray_available, status.portal_available
            );
            Ok(())
        }
        IpcResponse::Ok => Err(anyhow!("unexpected ok response to status request")),
        IpcResponse::Error { message } => {
            Err(anyhow!(message)).context("daemon rejected status request")
        }
    }
}
