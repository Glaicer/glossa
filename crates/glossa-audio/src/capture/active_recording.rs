use std::{sync::mpsc::SyncSender, thread::JoinHandle, time::Instant};

use async_trait::async_trait;
use camino::Utf8PathBuf;
use cpal::traits::StreamTrait;

use glossa_app::{ports::ActiveRecording, AppError};
use glossa_core::{CapturedAudio, SessionId};

/// Concrete active recording handle for the CPAL capture backend.
pub struct CpalActiveRecording {
    pub session_id: SessionId,
    pub path: Utf8PathBuf,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub started_at: Instant,
    pub stream: cpal::Stream,
    pub tx: Option<SyncSender<Vec<i16>>>,
    pub writer_handle: Option<JoinHandle<Result<(), String>>>,
}

#[async_trait(?Send)]
impl ActiveRecording for CpalActiveRecording {
    async fn stop(self: Box<Self>) -> Result<CapturedAudio, AppError> {
        finish_recording(*self)
    }

    async fn abort(self: Box<Self>) -> Result<(), AppError> {
        let _ = self.stream.pause();
        drop(self.tx);
        if let Some(writer_handle) = self.writer_handle {
            let _ = writer_handle.join();
        }
        Ok(())
    }
}

fn finish_recording(mut recording: CpalActiveRecording) -> Result<CapturedAudio, AppError> {
    let session_id = recording.session_id;
    let path = recording.path.clone();
    let sample_rate_hz = recording.sample_rate_hz;
    let channels = recording.channels;
    let duration_ms = recording.started_at.elapsed().as_millis() as u64;

    let stream = recording.stream;
    stream
        .pause()
        .map_err(|error| AppError::message(format!("failed to pause audio stream: {error}")))?;
    drop(recording.tx.take());
    drop(stream);

    if let Some(writer_handle) = recording.writer_handle.take() {
        let result = writer_handle
            .join()
            .map_err(|_| AppError::message("audio writer thread panicked"))?;
        if let Err(error) = result {
            return Err(AppError::message(error));
        }
    }

    Ok(CapturedAudio {
        session_id,
        path,
        duration_ms,
        sample_rate_hz,
        channels,
    })
}
