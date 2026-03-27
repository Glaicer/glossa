use std::{env, path::PathBuf};

use async_trait::async_trait;
use camino::{Utf8Path, Utf8PathBuf};
use tokio::fs;

use glossa_app::{ports::TempStore, AppError};
use glossa_core::{AudioConfig, AudioFormat, SessionId, WorkDir};

/// Temporary storage rooted in `$XDG_RUNTIME_DIR` or a configured directory.
#[derive(Debug, Clone)]
pub struct XdgTempStore {
    base_dir: Utf8PathBuf,
}

impl XdgTempStore {
    pub fn from_audio_config(config: &AudioConfig) -> Result<Self, AppError> {
        let base_dir = match &config.work_dir {
            WorkDir::Auto => {
                let runtime_dir = env::var("XDG_RUNTIME_DIR")
                    .ok()
                    .map(Utf8PathBuf::from)
                    .or_else(|| env::temp_dir().to_str().map(Utf8PathBuf::from))
                    .ok_or_else(|| {
                        AppError::message("could not determine a runtime directory for temp files")
                    })?;
                runtime_dir.join("glossa")
            }
            WorkDir::Path(path) => path.clone(),
        };

        Ok(Self { base_dir })
    }

    #[must_use]
    pub fn base_dir(&self) -> &Utf8Path {
        self.base_dir.as_path()
    }

    fn recording_path(&self, session_id: SessionId, format: AudioFormat) -> Utf8PathBuf {
        self.base_dir
            .join(format!("glossa-{session_id}.{}", format.extension()))
    }
}

#[async_trait]
impl TempStore for XdgTempStore {
    async fn create_recording_path(
        &self,
        session_id: SessionId,
        format: AudioFormat,
    ) -> Result<Utf8PathBuf, AppError> {
        fs::create_dir_all(self.base_dir())
            .await
            .map_err(|error| AppError::io("failed to create temp directory", error))?;
        Ok(self.recording_path(session_id, format))
    }

    async fn cleanup_session(&self, session_id: SessionId) -> Result<(), AppError> {
        fs::create_dir_all(self.base_dir())
            .await
            .map_err(|error| AppError::io("failed to create temp directory", error))?;
        let prefix = format!("glossa-{session_id}");
        let mut entries = fs::read_dir(self.base_dir())
            .await
            .map_err(|error| AppError::io("failed to read temp directory", error))?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|error| AppError::io("failed to iterate temp directory", error))?
        {
            let path = entry.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
            {
                let _ = fs::remove_file(path).await;
            }
        }
        Ok(())
    }

    async fn cleanup_stale_files(&self) -> Result<(), AppError> {
        fs::create_dir_all(self.base_dir())
            .await
            .map_err(|error| AppError::io("failed to create temp directory", error))?;
        let mut entries = fs::read_dir(self.base_dir())
            .await
            .map_err(|error| AppError::io("failed to read temp directory", error))?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|error| AppError::io("failed to iterate temp directory", error))?
        {
            let path: PathBuf = entry.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("glossa-"))
            {
                let _ = fs::remove_file(path).await;
            }
        }
        Ok(())
    }
}
