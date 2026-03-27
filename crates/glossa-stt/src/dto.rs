use serde::Deserialize;

/// Normal transcription response shape returned by OpenAI-compatible APIs.
#[derive(Debug, Deserialize)]
pub struct TranscriptionResponse {
    pub text: String,
}
