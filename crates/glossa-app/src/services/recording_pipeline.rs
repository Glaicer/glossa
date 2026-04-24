use std::{borrow::Cow, sync::Arc};

use tokio::{sync::mpsc, task};
use tracing::{info, warn};

use glossa_core::{AppConfig, CapturedAudio, SessionId};

use crate::{
    ports::{ClipboardWriter, PasteBackend, SilenceTrimmer, SttClient, TempStore},
    AppError,
};

/// Messages emitted back to the actor from background tasks.
#[derive(Debug)]
pub enum InternalEvent {
    ProcessingReady {
        session_id: SessionId,
        text: String,
    },
    ProcessingFinished {
        session_id: SessionId,
        outcome: CycleOutcome,
    },
    PasteFinished {
        session_id: SessionId,
        outcome: CycleOutcome,
    },
}

/// Final outcome of a capture/transcription/paste cycle.
#[derive(Debug, Clone)]
pub enum CycleOutcome {
    Completed,
    CompletedWithWarning(String),
    Failed(String),
}

/// Shared dependencies for processing and paste tasks.
#[derive(Clone)]
pub struct PipelineDependencies {
    pub config: Arc<AppConfig>,
    pub trimmer: Arc<dyn SilenceTrimmer>,
    pub stt_client: Arc<dyn SttClient>,
    pub clipboard: Arc<dyn ClipboardWriter>,
    pub paste: Arc<dyn PasteBackend>,
    pub temp_store: Arc<dyn TempStore>,
}

/// Spawns the non-paste processing steps for a captured recording.
pub fn spawn_processing_task(
    tx: mpsc::UnboundedSender<InternalEvent>,
    deps: PipelineDependencies,
    audio: CapturedAudio,
) {
    task::spawn(async move {
        let session_id = audio.session_id;
        let processed_audio = if deps.config.audio.trim_silence {
            match deps.trimmer.trim(&audio).await {
                Ok(trimmed) => trimmed,
                Err(error) => {
                    warn!(%session_id, error = %error, "silence trimming failed; continuing with the original file");
                    audio
                }
            }
        } else {
            audio
        };

        if processed_audio.duration_ms == 0 {
            let _ = deps.temp_store.cleanup_session(session_id).await;
            let _ = tx.send(InternalEvent::ProcessingFinished {
                session_id,
                outcome: CycleOutcome::CompletedWithWarning("recording was empty".into()),
            });
            return;
        }

        if processed_audio.duration_ms < deps.config.audio.min_duration_ms {
            let _ = deps.temp_store.cleanup_session(session_id).await;
            let _ = tx.send(InternalEvent::ProcessingFinished {
                session_id,
                outcome: CycleOutcome::CompletedWithWarning(
                    "recording was shorter than the configured minimum duration".into(),
                ),
            });
            return;
        }

        info!(%session_id, provider = deps.stt_client.provider_name(), "starting transcription");
        match deps.stt_client.transcribe(&processed_audio).await {
            Ok(text) => {
                let normalized = text.trim().replace("\r\n", "\n");
                if normalized.is_empty() {
                    let _ = deps.temp_store.cleanup_session(session_id).await;
                    let _ = tx.send(InternalEvent::ProcessingFinished {
                        session_id,
                        outcome: CycleOutcome::CompletedWithWarning(
                            "transcription returned empty text".into(),
                        ),
                    });
                } else {
                    info!(
                        %session_id,
                        provider = deps.stt_client.provider_name(),
                        text = %normalized,
                        "transcription completed"
                    );
                    let _ = tx.send(InternalEvent::ProcessingReady {
                        session_id,
                        text: normalized,
                    });
                }
            }
            Err(error) => {
                let _ = deps.temp_store.cleanup_session(session_id).await;
                let _ = tx.send(InternalEvent::ProcessingFinished {
                    session_id,
                    outcome: CycleOutcome::Failed(error.to_string()),
                });
            }
        }
    });
}

/// Spawns the final clipboard + paste phase for a transcription result.
pub fn spawn_paste_task(
    tx: mpsc::UnboundedSender<InternalEvent>,
    deps: PipelineDependencies,
    session_id: SessionId,
    text: String,
) {
    task::spawn(async move {
        let result = paste_cycle(&deps, session_id, &text).await;
        let outcome = match result {
            Ok(outcome) => outcome,
            Err(error) => CycleOutcome::Failed(error.to_string()),
        };

        let _ = deps.temp_store.cleanup_session(session_id).await;
        let _ = tx.send(InternalEvent::PasteFinished {
            session_id,
            outcome,
        });
    });
}

async fn paste_cycle(
    deps: &PipelineDependencies,
    session_id: SessionId,
    text: &str,
) -> Result<CycleOutcome, AppError> {
    let clipboard_text = if deps.config.paste.append_space && !text.is_empty() {
        Cow::Owned(format!("{text} "))
    } else {
        Cow::Borrowed(text)
    };

    info!(
        %session_id,
        text_len = clipboard_text.len(),
        "writing transcription to clipboard"
    );
    deps.clipboard.set_text(clipboard_text.as_ref()).await?;
    info!(
        %session_id,
        paste_mode = ?deps.config.paste.mode,
        "attempting paste via configured backend"
    );
    match deps.paste.paste(deps.config.paste.mode).await {
        Ok(()) => Ok(CycleOutcome::Completed),
        Err(error) => Ok(CycleOutcome::CompletedWithWarning(format!(
            "paste failed after clipboard write for session {session_id}: {error}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use camino::Utf8PathBuf;

    use glossa_core::{AudioFormat, CapturedAudio, PasteMode, SessionId};

    use super::{paste_cycle, CycleOutcome, PipelineDependencies};
    use crate::{
        ports::{ClipboardWriter, PasteBackend, SilenceTrimmer, SttClient, TempStore},
        AppError,
    };

    struct RecordingClipboard {
        text: Mutex<Vec<String>>,
    }

    impl RecordingClipboard {
        fn new() -> Self {
            Self {
                text: Mutex::new(Vec::new()),
            }
        }

        fn recorded(&self) -> Vec<String> {
            self.text.lock().expect("clipboard lock").clone()
        }
    }

    #[async_trait]
    impl ClipboardWriter for RecordingClipboard {
        async fn set_text(&self, text: &str) -> Result<(), AppError> {
            self.text
                .lock()
                .expect("clipboard lock")
                .push(text.to_owned());
            Ok(())
        }
    }

    struct RecordingPaste {
        modes: Mutex<Vec<PasteMode>>,
    }

    impl RecordingPaste {
        fn new() -> Self {
            Self {
                modes: Mutex::new(Vec::new()),
            }
        }

        fn recorded(&self) -> Vec<PasteMode> {
            self.modes.lock().expect("paste lock").clone()
        }
    }

    #[async_trait]
    impl PasteBackend for RecordingPaste {
        async fn paste(&self, mode: PasteMode) -> Result<(), AppError> {
            self.modes.lock().expect("paste lock").push(mode);
            Ok(())
        }
    }

    struct NoopTrimmer;

    #[async_trait]
    impl SilenceTrimmer for NoopTrimmer {
        async fn trim(&self, input: &CapturedAudio) -> Result<CapturedAudio, AppError> {
            Ok(input.clone())
        }
    }

    struct NoopSttClient;

    #[async_trait]
    impl SttClient for NoopSttClient {
        fn provider_name(&self) -> &'static str {
            "test"
        }

        async fn transcribe(&self, _audio: &CapturedAudio) -> Result<String, AppError> {
            Ok(String::new())
        }
    }

    struct NoopTempStore;

    #[async_trait]
    impl TempStore for NoopTempStore {
        async fn create_recording_path(
            &self,
            _session_id: SessionId,
            _format: AudioFormat,
        ) -> Result<Utf8PathBuf, AppError> {
            Err(AppError::message("not used in paste_cycle tests"))
        }

        async fn cleanup_session(&self, _session_id: SessionId) -> Result<(), AppError> {
            Ok(())
        }

        async fn purge_session(&self, _session_id: SessionId) -> Result<(), AppError> {
            Ok(())
        }

        async fn cleanup_stale_files(&self) -> Result<(), AppError> {
            Ok(())
        }
    }

    fn test_dependencies(
        append_space: bool,
    ) -> (
        PipelineDependencies,
        Arc<RecordingClipboard>,
        Arc<RecordingPaste>,
    ) {
        let clipboard = Arc::new(RecordingClipboard::new());
        let paste = Arc::new(RecordingPaste::new());
        let mut config = glossa_core::AppConfig::default();
        config.paste.append_space = append_space;

        (
            PipelineDependencies {
                config: Arc::new(config),
                trimmer: Arc::new(NoopTrimmer),
                stt_client: Arc::new(NoopSttClient),
                clipboard: clipboard.clone(),
                paste: paste.clone(),
                temp_store: Arc::new(NoopTempStore),
            },
            clipboard,
            paste,
        )
    }

    #[tokio::test]
    async fn paste_cycle_should_append_a_space_before_writing_to_clipboard_when_enabled() {
        let (deps, clipboard, paste) = test_dependencies(true);
        let session_id = SessionId::new();

        let outcome = paste_cycle(&deps, session_id, "hello")
            .await
            .expect("paste cycle should succeed");

        assert!(matches!(outcome, CycleOutcome::Completed));
        assert_eq!(clipboard.recorded(), vec!["hello ".to_owned()]);
        assert_eq!(paste.recorded(), vec![PasteMode::CtrlV]);
    }

    #[tokio::test]
    async fn paste_cycle_should_leave_text_unchanged_when_append_space_is_disabled() {
        let (deps, clipboard, paste) = test_dependencies(false);
        let session_id = SessionId::new();

        let outcome = paste_cycle(&deps, session_id, "hello")
            .await
            .expect("paste cycle should succeed");

        assert!(matches!(outcome, CycleOutcome::Completed));
        assert_eq!(clipboard.recorded(), vec!["hello".to_owned()]);
        assert_eq!(paste.recorded(), vec![PasteMode::CtrlV]);
    }
}
