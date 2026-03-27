use std::sync::Arc;

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
    deps.clipboard.set_text(text).await?;
    match deps.paste.paste(deps.config.paste.mode).await {
        Ok(()) => Ok(CycleOutcome::Completed),
        Err(error) => Ok(CycleOutcome::CompletedWithWarning(format!(
            "paste failed after clipboard write for session {session_id}: {error}"
        ))),
    }
}
