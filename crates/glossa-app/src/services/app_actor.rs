use std::{sync::Arc, time::Instant};

use tokio::sync::mpsc;
use tracing::{error, info, warn};

use glossa_core::{AppCommand, AppConfig, AppState, PastingState, RecordSpec, SessionId};

use crate::{
    machine::{reduce, Action},
    ports::{
        ActiveRecording, AudioCapture, ClipboardWriter, CuePlayer, PasteBackend, SilenceTrimmer,
        SttClient, TempStore, TrayPort, TrayState,
    },
    services::{
        command_router::AppHandle,
        recording_pipeline::{
            spawn_paste_task, spawn_processing_task, CycleOutcome, InternalEvent,
            PipelineDependencies,
        },
        status_store::StatusStore,
    },
    AppError,
};

/// Runtime dependencies owned by the application actor.
#[derive(Clone)]
pub struct AppDependencies {
    pub audio_capture: Arc<dyn AudioCapture>,
    pub trimmer: Arc<dyn SilenceTrimmer>,
    pub cue_player: Arc<dyn CuePlayer>,
    pub stt_client: Arc<dyn SttClient>,
    pub clipboard: Arc<dyn ClipboardWriter>,
    pub paste: Arc<dyn PasteBackend>,
    pub tray: Arc<dyn TrayPort>,
    pub temp_store: Arc<dyn TempStore>,
}

/// Main daemon actor that serializes all state transitions.
pub struct AppActor {
    config: Arc<AppConfig>,
    deps: AppDependencies,
    state: AppState,
    active_recording: Option<Box<dyn ActiveRecording>>,
    command_rx: mpsc::UnboundedReceiver<AppCommand>,
    internal_tx: mpsc::UnboundedSender<InternalEvent>,
    internal_rx: mpsc::UnboundedReceiver<InternalEvent>,
    status_store: StatusStore,
}

/// Reason why the actor stopped processing commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorExit {
    Restart,
    Shutdown,
}

impl AppActor {
    /// Creates a new actor plus the handle that external components can use.
    #[must_use]
    pub fn new(config: AppConfig, deps: AppDependencies) -> (Self, AppHandle) {
        let config = Arc::new(config);
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (internal_tx, internal_rx) = mpsc::unbounded_channel();
        let status_store = StatusStore::new(config.initial_status());
        let handle = AppHandle::new(command_tx, status_store.clone());
        let actor = Self {
            config,
            deps,
            state: AppState::Idle,
            active_recording: None,
            command_rx,
            internal_tx,
            internal_rx,
            status_store,
        };
        (actor, handle)
    }

    /// Runs the actor until shutdown is requested.
    pub async fn run(mut self) -> Result<ActorExit, AppError> {
        self.deps.temp_store.cleanup_stale_files().await?;
        self.set_tray_state(TrayState::Idle).await;
        self.publish_status();

        loop {
            tokio::select! {
                maybe_command = self.command_rx.recv() => {
                    let Some(command) = maybe_command else {
                        break;
                    };
                    if let Some(exit) = self.handle_command(command).await? {
                        return Ok(exit);
                    }
                }
                maybe_event = self.internal_rx.recv() => {
                    let Some(event) = maybe_event else {
                        break;
                    };
                    self.handle_internal_event(event).await?;
                }
            }
        }

        Ok(ActorExit::Shutdown)
    }

    async fn handle_command(&mut self, command: AppCommand) -> Result<Option<ActorExit>, AppError> {
        info!(state = ?self.state.kind(), command = ?command, "received command");
        let decision = reduce(&self.state, &command)?;
        self.state = decision.next_state;
        self.publish_status();

        for action in decision.actions {
            match action {
                Action::StartRecording { session_id } => self.start_recording(session_id).await?,
                Action::StopRecording { session_id } => self.stop_recording(session_id).await?,
                Action::SetTrayIdle => self.set_tray_state(TrayState::Idle).await,
                Action::SetTrayRecording => self.set_tray_state(TrayState::Recording).await,
                Action::SetTrayProcessing => self.set_tray_state(TrayState::Processing).await,
                Action::PlayStartCue => {
                    if let Err(error) = self.deps.cue_player.play_start().await {
                        warn!(error = %error, "failed to play start cue");
                    }
                }
                Action::PlayStopCue => {
                    if let Err(error) = self.deps.cue_player.play_stop().await {
                        warn!(error = %error, "failed to play stop cue");
                    }
                }
                Action::Ignore { reason } => {
                    info!(reason, "ignored command");
                }
                Action::Restart => {
                    self.shutdown().await?;
                    return Ok(Some(ActorExit::Restart));
                }
                Action::Shutdown => {
                    self.shutdown().await?;
                    return Ok(Some(ActorExit::Shutdown));
                }
            }
        }

        Ok(None)
    }

    async fn handle_internal_event(&mut self, event: InternalEvent) -> Result<(), AppError> {
        match event {
            InternalEvent::ProcessingReady { session_id, text } => {
                if !matches!(
                    self.state,
                    AppState::Processing(glossa_core::ProcessingState { session_id: current })
                        if current == session_id
                ) {
                    return Ok(());
                }

                self.state = AppState::Pasting(PastingState {
                    session_id,
                    text_len: text.len(),
                });
                self.publish_status();
                spawn_paste_task(
                    self.internal_tx.clone(),
                    self.pipeline_dependencies(),
                    session_id,
                    text,
                );
            }
            InternalEvent::ProcessingFinished {
                session_id,
                outcome,
            } => {
                self.finish_cycle(session_id, outcome).await;
            }
            InternalEvent::PasteFinished {
                session_id,
                outcome,
            } => {
                self.finish_cycle(session_id, outcome).await;
            }
        }
        Ok(())
    }

    async fn start_recording(&mut self, session_id: SessionId) -> Result<(), AppError> {
        let startup_started_at = Instant::now();
        let path_started_at = Instant::now();
        let path = self
            .deps
            .temp_store
            .create_recording_path(session_id, self.config.audio.format)
            .await?;
        let path_prep_ms = path_started_at.elapsed().as_millis();
        let spec = RecordSpec {
            sample_rate_hz: self.config.audio.sample_rate_hz,
            channels: self.config.audio.channels,
            format: self.config.audio.format,
            max_duration_sec: self.config.audio.max_duration_sec,
        };
        let capture_started_at = Instant::now();
        match self
            .deps
            .audio_capture
            .start(session_id, spec, path.as_path())
            .await
        {
            Ok(recording) => {
                self.active_recording = Some(recording);
                info!(
                    %session_id,
                    path = %path,
                    path_prep_ms,
                    capture_startup_ms = capture_started_at.elapsed().as_millis(),
                    total_startup_ms = startup_started_at.elapsed().as_millis(),
                    "recording started"
                );
            }
            Err(error) => {
                error!(
                    %session_id,
                    error = %error,
                    path_prep_ms,
                    capture_startup_ms = capture_started_at.elapsed().as_millis(),
                    total_startup_ms = startup_started_at.elapsed().as_millis(),
                    "failed to start recording"
                );
                self.state = AppState::Idle;
                self.publish_status();
                self.set_tray_state(TrayState::Idle).await;
                let _ = self.deps.temp_store.cleanup_session(session_id).await;
                let _ = self.deps.tray.show_error(&error.to_string()).await;
            }
        }
        Ok(())
    }

    async fn stop_recording(&mut self, session_id: SessionId) -> Result<(), AppError> {
        let Some(recording) = self.active_recording.take() else {
            warn!(%session_id, "stop requested without an active recording");
            self.state = AppState::Idle;
            self.publish_status();
            self.set_tray_state(TrayState::Idle).await;
            return Ok(());
        };

        match recording.stop().await {
            Ok(audio) => {
                info!(%session_id, duration_ms = audio.duration_ms, path = %audio.path, "recording stopped");
                spawn_processing_task(
                    self.internal_tx.clone(),
                    self.pipeline_dependencies(),
                    audio,
                );
            }
            Err(error) => {
                error!(%session_id, error = %error, "failed to stop recording");
                self.finish_cycle(session_id, CycleOutcome::Failed(error.to_string()))
                    .await;
            }
        }
        Ok(())
    }

    async fn finish_cycle(&mut self, session_id: SessionId, outcome: CycleOutcome) {
        match &outcome {
            CycleOutcome::Completed => info!(%session_id, "cycle completed"),
            CycleOutcome::CompletedWithWarning(reason) => {
                warn!(%session_id, reason, "cycle completed with warning");
            }
            CycleOutcome::Failed(reason) => {
                error!(%session_id, reason, "cycle failed");
                let _ = self.deps.tray.show_error(reason).await;
            }
        }

        self.state = AppState::Idle;
        self.publish_status();
        self.set_tray_state(TrayState::Idle).await;
    }

    async fn shutdown(&mut self) -> Result<(), AppError> {
        if let Some(recording) = self.active_recording.take() {
            let _ = recording.abort().await;
        }
        self.state = AppState::ShuttingDown;
        self.publish_status();
        self.set_tray_state(TrayState::Idle).await;
        Ok(())
    }

    fn publish_status(&self) {
        let mut status = self.config.initial_status();
        status.state = self.state.kind();
        self.status_store.update(status);
    }

    async fn set_tray_state(&self, state: TrayState) {
        if let Err(error) = self.deps.tray.set_state(state).await {
            warn!(error = %error, "failed to update tray state");
        }
    }

    fn pipeline_dependencies(&self) -> PipelineDependencies {
        PipelineDependencies {
            config: Arc::clone(&self.config),
            trimmer: Arc::clone(&self.deps.trimmer),
            stt_client: Arc::clone(&self.deps.stt_client),
            clipboard: Arc::clone(&self.deps.clipboard),
            paste: Arc::clone(&self.deps.paste),
            temp_store: Arc::clone(&self.deps.temp_store),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use camino::{Utf8Path, Utf8PathBuf};

    use glossa_core::{AudioFormat, CapturedAudio, CommandOrigin};

    use super::*;
    use crate::ports::{
        AudioCapture, ClipboardWriter, CuePlayer, PasteBackend, SilenceTrimmer, SttClient,
        TempStore,
    };

    #[derive(Debug)]
    struct FakeAudioCapture {
        started: Arc<Mutex<bool>>,
    }

    #[derive(Debug)]
    struct OrderedAudioCapture {
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    struct FakeRecording;

    #[async_trait]
    impl AudioCapture for FakeAudioCapture {
        async fn start(
            &self,
            session_id: SessionId,
            spec: RecordSpec,
            path: &Utf8Path,
        ) -> Result<Box<dyn ActiveRecording>, AppError> {
            let _ = (session_id, spec, path);
            *self.started.lock().expect("mutex should not be poisoned") = true;
            Ok(Box::new(FakeRecording))
        }
    }

    #[async_trait]
    impl AudioCapture for OrderedAudioCapture {
        async fn start(
            &self,
            session_id: SessionId,
            spec: RecordSpec,
            path: &Utf8Path,
        ) -> Result<Box<dyn ActiveRecording>, AppError> {
            let _ = (session_id, spec, path);
            self.events
                .lock()
                .expect("mutex should not be poisoned")
                .push("start-recording");
            Ok(Box::new(FakeRecording))
        }
    }

    #[async_trait(?Send)]
    impl ActiveRecording for FakeRecording {
        async fn stop(self: Box<Self>) -> Result<CapturedAudio, AppError> {
            Ok(CapturedAudio {
                session_id: SessionId::new(),
                path: Utf8PathBuf::from("/tmp/test.wav"),
                duration_ms: 500,
                sample_rate_hz: 16_000,
                channels: 1,
            })
        }

        async fn abort(self: Box<Self>) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct FakeTrimmer;

    #[async_trait]
    impl SilenceTrimmer for FakeTrimmer {
        async fn trim(&self, input: &CapturedAudio) -> Result<CapturedAudio, AppError> {
            Ok(input.clone())
        }
    }

    #[derive(Debug, Default)]
    struct FakeCuePlayer;

    #[derive(Debug)]
    struct OrderedCuePlayer {
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    #[async_trait]
    impl CuePlayer for FakeCuePlayer {
        async fn play_start(&self) -> Result<(), AppError> {
            Ok(())
        }

        async fn play_stop(&self) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[async_trait]
    impl CuePlayer for OrderedCuePlayer {
        async fn play_start(&self) -> Result<(), AppError> {
            self.events
                .lock()
                .expect("mutex should not be poisoned")
                .push("play-start-cue");
            Ok(())
        }

        async fn play_stop(&self) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct FakeSttClient;

    #[async_trait]
    impl SttClient for FakeSttClient {
        fn provider_name(&self) -> &'static str {
            "fake"
        }

        async fn transcribe(&self, _audio: &CapturedAudio) -> Result<String, AppError> {
            Ok("hello".into())
        }
    }

    #[derive(Debug, Default)]
    struct FakeClipboard;

    #[async_trait]
    impl ClipboardWriter for FakeClipboard {
        async fn set_text(&self, _text: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct FakePaste;

    #[async_trait]
    impl PasteBackend for FakePaste {
        async fn paste(&self, _mode: glossa_core::PasteMode) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct FakeTempStore;

    #[async_trait]
    impl TempStore for FakeTempStore {
        async fn create_recording_path(
            &self,
            session_id: SessionId,
            format: AudioFormat,
        ) -> Result<Utf8PathBuf, AppError> {
            Ok(Utf8PathBuf::from(format!(
                "/tmp/{session_id}.{}",
                format.extension()
            )))
        }

        async fn cleanup_session(&self, _session_id: SessionId) -> Result<(), AppError> {
            Ok(())
        }

        async fn cleanup_stale_files(&self) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn actor_should_expose_initial_status() {
        let deps = AppDependencies {
            audio_capture: Arc::new(FakeAudioCapture {
                started: Arc::new(Mutex::new(false)),
            }),
            trimmer: Arc::new(FakeTrimmer),
            cue_player: Arc::new(FakeCuePlayer),
            stt_client: Arc::new(FakeSttClient),
            clipboard: Arc::new(FakeClipboard),
            paste: Arc::new(FakePaste),
            tray: Arc::new(crate::ports::NullTrayPort),
            temp_store: Arc::new(FakeTempStore),
        };
        let (_actor, handle) = AppActor::new(AppConfig::default(), deps);
        assert_eq!(handle.status().state, glossa_core::AppStateKind::Idle);
    }

    #[tokio::test]
    async fn actor_should_start_recording_before_playing_start_cue() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let deps = AppDependencies {
            audio_capture: Arc::new(OrderedAudioCapture {
                events: Arc::clone(&events),
            }),
            trimmer: Arc::new(FakeTrimmer),
            cue_player: Arc::new(OrderedCuePlayer {
                events: Arc::clone(&events),
            }),
            stt_client: Arc::new(FakeSttClient),
            clipboard: Arc::new(FakeClipboard),
            paste: Arc::new(FakePaste),
            tray: Arc::new(crate::ports::NullTrayPort),
            temp_store: Arc::new(FakeTempStore),
        };
        let (mut actor, _handle) = AppActor::new(AppConfig::default(), deps);

        let exit = actor
            .handle_command(AppCommand::ToggleRecording {
                origin: CommandOrigin::CliControl,
            })
            .await
            .expect("command should succeed");

        assert_eq!(exit, None);
        assert_eq!(
            *events.lock().expect("mutex should not be poisoned"),
            vec!["start-recording", "play-start-cue"]
        );
    }
}
