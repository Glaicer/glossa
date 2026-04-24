use anyhow::{anyhow, Context};

use glossa_platform_linux::ipc::{IpcClient, IpcRequest, IpcResponse};

use crate::{bootstrap::resolve_socket_path, cli::CtlCommand};

pub async fn run(
    config_path: Option<std::path::PathBuf>,
    command: CtlCommand,
) -> anyhow::Result<()> {
    let socket_path = resolve_socket_path(config_path).await?;
    let client = IpcClient::new(socket_path);
    let request = ipc_request_for_ctl_command(command);

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

fn ipc_request_for_ctl_command(command: CtlCommand) -> IpcRequest {
    match command {
        CtlCommand::Toggle => IpcRequest::Toggle,
        CtlCommand::Stream => IpcRequest::Stream,
        CtlCommand::Shutdown => IpcRequest::Shutdown,
    }
}

#[cfg(test)]
mod tests {
    use super::ipc_request_for_ctl_command;
    use crate::cli::CtlCommand;
    use glossa_platform_linux::ipc::IpcRequest;

    #[test]
    fn ctl_stream_should_send_stream_ipc_request() {
        assert_eq!(
            ipc_request_for_ctl_command(CtlCommand::Stream),
            IpcRequest::Stream
        );
    }
}
