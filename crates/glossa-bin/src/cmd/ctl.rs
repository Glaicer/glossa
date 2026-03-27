use anyhow::{anyhow, Context};

use glossa_platform_linux::ipc::{IpcClient, IpcRequest, IpcResponse};

use crate::{bootstrap::resolve_socket_path, cli::CtlCommand};

pub async fn run(
    config_path: Option<std::path::PathBuf>,
    command: CtlCommand,
) -> anyhow::Result<()> {
    let socket_path = resolve_socket_path(config_path).await?;
    let client = IpcClient::new(socket_path);
    let request = match command {
        CtlCommand::Toggle => IpcRequest::Toggle,
        CtlCommand::Shutdown => IpcRequest::Shutdown,
    };

    let response = client.request(request).await?;
    match response {
        IpcResponse::Ok => {
            println!("OK");
            Ok(())
        }
        IpcResponse::Status { .. } => Err(anyhow!("unexpected status response to ctl command")),
        IpcResponse::Error { message } => {
            Err(anyhow!(message)).context("daemon rejected ctl command")
        }
    }
}
