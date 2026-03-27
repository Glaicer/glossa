use std::{fs::File, io::BufReader};

use async_trait::async_trait;
use camino::Utf8PathBuf;
use rodio::{Decoder, OutputStream, Sink};

use glossa_app::{ports::CuePlayer, AppError};

/// Cue player backed by Rodio for start/stop sound effects.
#[derive(Debug, Clone)]
pub struct RodioCuePlayer {
    start_sound: Utf8PathBuf,
    stop_sound: Utf8PathBuf,
}

impl RodioCuePlayer {
    #[must_use]
    pub fn new(start_sound: Utf8PathBuf, stop_sound: Utf8PathBuf) -> Self {
        Self {
            start_sound,
            stop_sound,
        }
    }
}

#[async_trait]
impl CuePlayer for RodioCuePlayer {
    async fn play_start(&self) -> Result<(), AppError> {
        let path = self.start_sound.clone();
        tokio::task::spawn_blocking(move || play_file(path))
            .await
            .map_err(|error| AppError::message(format!("failed to join cue task: {error}")))?
    }

    async fn play_stop(&self) -> Result<(), AppError> {
        let path = self.stop_sound.clone();
        tokio::task::spawn_blocking(move || play_file(path))
            .await
            .map_err(|error| AppError::message(format!("failed to join cue task: {error}")))?
    }
}

fn play_file(path: Utf8PathBuf) -> Result<(), AppError> {
    let file = File::open(path.as_std_path())
        .map_err(|error| AppError::io("failed to open cue sound file", error))?;
    let (_stream, stream_handle) = OutputStream::try_default().map_err(|error| {
        AppError::message(format!("failed to open audio output stream: {error}"))
    })?;
    let sink = Sink::try_new(&stream_handle)
        .map_err(|error| AppError::message(format!("failed to create audio sink: {error}")))?;
    let source = Decoder::new(BufReader::new(file))
        .map_err(|error| AppError::message(format!("failed to decode cue sound: {error}")))?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}
