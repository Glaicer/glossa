use camino::Utf8PathBuf;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

use glossa_app::AppError;

use super::{IpcRequest, IpcResponse};

/// Unix-domain-socket IPC client for `glossa ctl` and `glossa status`.
#[derive(Debug, Clone)]
pub struct IpcClient {
    socket_path: Utf8PathBuf,
}

impl IpcClient {
    #[must_use]
    pub fn new(socket_path: Utf8PathBuf) -> Self {
        Self { socket_path }
    }

    pub async fn request(&self, request: IpcRequest) -> Result<IpcResponse, AppError> {
        let mut stream = UnixStream::connect(self.socket_path.as_std_path())
            .await
            .map_err(|error| AppError::io("failed to connect to the daemon socket", error))?;
        let payload = serde_json::to_vec(&request)
            .map_err(|error| AppError::message(format!("failed to encode ipc request: {error}")))?;
        stream
            .write_all(&payload)
            .await
            .map_err(|error| AppError::io("failed to write ipc request", error))?;
        stream
            .write_all(b"\n")
            .await
            .map_err(|error| AppError::io("failed to terminate ipc request", error))?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|error| AppError::io("failed to read ipc response", error))?;

        serde_json::from_str(&line)
            .map_err(|error| AppError::message(format!("failed to decode ipc response: {error}")))
    }
}
