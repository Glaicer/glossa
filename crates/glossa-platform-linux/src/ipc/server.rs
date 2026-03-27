use camino::Utf8PathBuf;
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};
use tracing::{error, info};

use glossa_app::{AppError, AppHandle};
use glossa_core::{AppCommand, CommandOrigin};

use super::{IpcRequest, IpcResponse};

/// Unix-domain-socket IPC server for daemon control and status inspection.
#[derive(Debug, Clone)]
pub struct IpcServer {
    socket_path: Utf8PathBuf,
    handle: AppHandle,
}

impl IpcServer {
    #[must_use]
    pub fn new(socket_path: Utf8PathBuf, handle: AppHandle) -> Self {
        Self {
            socket_path,
            handle,
        }
    }

    pub async fn run(self) -> Result<(), AppError> {
        if let Some(parent) = self.socket_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|error| AppError::io("failed to create socket directory", error))?;
        }
        if fs::try_exists(self.socket_path.as_std_path())
            .await
            .map_err(|error| AppError::io("failed to check socket path", error))?
        {
            let _ = fs::remove_file(self.socket_path.as_std_path()).await;
        }

        let listener = UnixListener::bind(self.socket_path.as_std_path())
            .map_err(|error| AppError::io("failed to bind daemon socket", error))?;
        info!(path = %self.socket_path, "ipc server listening");

        loop {
            let (stream, _) = listener
                .accept()
                .await
                .map_err(|error| AppError::io("failed to accept ipc client", error))?;
            let handle = self.handle.clone();
            tokio::spawn(async move {
                if let Err(error) = handle_client(stream, handle).await {
                    error!(error = %error, "ipc client handling failed");
                }
            });
        }
    }
}

async fn handle_client(stream: UnixStream, handle: AppHandle) -> Result<(), AppError> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|error| AppError::io("failed to read ipc request", error))?;

    let request: IpcRequest = serde_json::from_str(&line)
        .map_err(|error| AppError::message(format!("failed to decode ipc request: {error}")))?;
    let response = match request {
        IpcRequest::Toggle => {
            handle.send(AppCommand::ToggleRecording {
                origin: CommandOrigin::CliControl,
            })?;
            IpcResponse::Ok
        }
        IpcRequest::Status => IpcResponse::Status {
            status: handle.status(),
        },
        IpcRequest::Shutdown => {
            handle.send(AppCommand::Shutdown {
                origin: CommandOrigin::CliControl,
            })?;
            IpcResponse::Ok
        }
    };

    let payload = serde_json::to_vec(&response)
        .map_err(|error| AppError::message(format!("failed to encode ipc response: {error}")))?;
    let mut stream = reader.into_inner();
    stream
        .write_all(&payload)
        .await
        .map_err(|error| AppError::io("failed to write ipc response", error))?;
    stream
        .write_all(b"\n")
        .await
        .map_err(|error| AppError::io("failed to terminate ipc response", error))?;
    Ok(())
}
