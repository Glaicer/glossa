use std::{fs::File, io::BufReader, sync::mpsc, thread, time::Instant};

use async_trait::async_trait;
use camino::Utf8PathBuf;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use tokio::sync::oneshot;

use glossa_app::{ports::CuePlayer, AppError};

type OpenAudioFn = dyn FnMut() -> Result<RodioAudio, AppError> + Send;

/// Cue player backed by Rodio for start/stop sound effects.
#[derive(Debug, Clone)]
pub struct RodioCuePlayer {
    start_sound: Utf8PathBuf,
    stop_sound: Utf8PathBuf,
    request_tx: mpsc::Sender<PlayRequest>,
}

#[derive(Debug)]
struct PlayRequest {
    cue: &'static str,
    path: Utf8PathBuf,
    done_tx: oneshot::Sender<()>,
}

struct RodioAudio {
    _stream: OutputStream,
    handle: OutputStreamHandle,
}

impl RodioCuePlayer {
    #[must_use]
    pub fn new(start_sound: Utf8PathBuf, stop_sound: Utf8PathBuf) -> Self {
        Self::from_open_audio(start_sound, stop_sound, Box::new(Self::open_audio))
    }

    #[cfg(test)]
    fn new_with_open_audio<F>(
        start_sound: Utf8PathBuf,
        stop_sound: Utf8PathBuf,
        open_audio: F,
    ) -> Self
    where
        F: FnMut() -> Result<RodioAudio, AppError> + Send + 'static,
    {
        Self::from_open_audio(start_sound, stop_sound, Box::new(open_audio))
    }

    fn from_open_audio(
        start_sound: Utf8PathBuf,
        stop_sound: Utf8PathBuf,
        mut open_audio: Box<OpenAudioFn>,
    ) -> Self {
        let (request_tx, request_rx) = mpsc::channel::<PlayRequest>();
        thread::spawn(move || {
            let mut audio = match open_audio.as_mut()() {
                Ok(audio) => Some(audio),
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        "cue audio unavailable at startup; cue playback will retry later"
                    );
                    None
                }
            };

            while let Ok(request) = request_rx.recv() {
                play_file_with_reopen_once(
                    &mut audio,
                    open_audio.as_mut(),
                    request.cue,
                    &request.path,
                );
                let _ = request.done_tx.send(());
            }
        });

        Self {
            start_sound,
            stop_sound,
            request_tx,
        }
    }

    fn open_audio() -> Result<RodioAudio, AppError> {
        let started_at = Instant::now();
        let (_stream, handle) = OutputStream::try_default().map_err(|error| {
            AppError::message(format!("failed to open audio output stream: {error}"))
        })?;
        tracing::info!(
            audio_init_ms = started_at.elapsed().as_millis(),
            "cue audio output stream initialized"
        );

        Ok(RodioAudio { _stream, handle })
    }

    async fn play(&self, cue: &'static str, path: Utf8PathBuf) -> Result<(), AppError> {
        let (done_tx, done_rx) = oneshot::channel();
        self.request_tx
            .send(PlayRequest { cue, path, done_tx })
            .map_err(|_| AppError::message("cue audio worker is unavailable"))?;
        done_rx
            .await
            .map_err(|_| AppError::message("cue audio worker stopped before playback completed"))?;
        Ok(())
    }
}

#[async_trait]
impl CuePlayer for RodioCuePlayer {
    async fn play_start(&self) -> Result<(), AppError> {
        self.play("start", self.start_sound.clone()).await
    }

    async fn play_stop(&self) -> Result<(), AppError> {
        self.play("stop", self.stop_sound.clone()).await
    }
}

fn play_file_with_reopen_once(
    audio: &mut Option<RodioAudio>,
    open_audio: &mut OpenAudioFn,
    cue: &'static str,
    path: &Utf8PathBuf,
) {
    if let Err(first_error) = play_file_once(audio, cue, path) {
        tracing::warn!(
            cue,
            path = %path,
            error = %first_error,
            "cue playback failed; reopening audio output stream and retrying once"
        );
        reopen_audio(audio, open_audio);

        if let Err(second_error) = play_file_once(audio, cue, path) {
            tracing::warn!(
                cue,
                path = %path,
                error = %second_error,
                "cue playback failed after reopening audio output stream; skipping cue"
            );
        }
    }
}

fn play_file_once(
    audio: &mut Option<RodioAudio>,
    cue: &'static str,
    path: &Utf8PathBuf,
) -> Result<(), AppError> {
    let playback_started_at = Instant::now();
    let file_open_started_at = Instant::now();
    let file = File::open(path.as_std_path())
        .map_err(|error| AppError::io("failed to open cue sound file", error))?;
    let file_open_ms = file_open_started_at.elapsed().as_millis();
    let audio = audio
        .as_ref()
        .ok_or_else(|| AppError::message("cue audio output stream is unavailable"))?;
    let sink_started_at = Instant::now();
    let sink = Sink::try_new(&audio.handle)
        .map_err(|error| AppError::message(format!("failed to create audio sink: {error}")))?;
    let sink_create_ms = sink_started_at.elapsed().as_millis();
    let decode_started_at = Instant::now();
    let source = Decoder::new(BufReader::new(file))
        .map_err(|error| AppError::message(format!("failed to decode cue sound: {error}")))?;
    let decode_ms = decode_started_at.elapsed().as_millis();
    sink.append(source);
    tracing::info!(
        cue,
        path = %path,
        file_open_ms,
        sink_create_ms,
        decode_ms,
        enqueue_ms = playback_started_at.elapsed().as_millis(),
        "cue playback enqueued"
    );
    sink.sleep_until_end();
    tracing::info!(
        cue,
        path = %path,
        total_playback_ms = playback_started_at.elapsed().as_millis(),
        "cue playback finished"
    );
    Ok(())
}

fn reopen_audio(audio: &mut Option<RodioAudio>, open_audio: &mut OpenAudioFn) {
    match open_audio() {
        Ok(reopened_audio) => {
            *audio = Some(reopened_audio);
        }
        Err(error) => {
            tracing::warn!(error = %error, "failed to reopen cue audio output stream");
            *audio = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use super::*;
    use crate::cue::CuePlayerBackend;

    #[tokio::test]
    async fn disabled_backend_should_noop() {
        let backend = CuePlayerBackend::from_config(
            false,
            Utf8PathBuf::from("/tmp/start.wav"),
            Utf8PathBuf::from("/tmp/stop.wav"),
        );

        backend
            .play_start()
            .await
            .expect("disabled backend should ignore start cue");
        backend
            .play_stop()
            .await
            .expect("disabled backend should ignore stop cue");

        assert!(matches!(backend, CuePlayerBackend::Disabled));
    }

    #[tokio::test]
    async fn play_start_should_retry_audio_reopen_once_and_still_succeed() {
        let open_calls = Arc::new(AtomicUsize::new(0));
        let player = RodioCuePlayer::new_with_open_audio(
            Utf8PathBuf::from("/tmp/missing-start.wav"),
            Utf8PathBuf::from("/tmp/missing-stop.wav"),
            {
                let open_calls = Arc::clone(&open_calls);
                move || {
                    open_calls.fetch_add(1, Ordering::SeqCst);
                    Err(AppError::message("audio unavailable"))
                }
            },
        );

        player
            .play_start()
            .await
            .expect("cue playback failure should stay best-effort");

        assert_eq!(open_calls.load(Ordering::SeqCst), 2);
    }
}
