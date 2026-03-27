use std::sync::Arc;

use async_trait::async_trait;
use reqwest::multipart::{Form, Part};
use tokio::fs;
use tracing::debug;

use glossa_app::{ports::SttClient, AppError};
use glossa_core::{CapturedAudio, ProviderConfig, ProviderKind};

use crate::dto::TranscriptionResponse;

/// Shared OpenAI-style multipart transcription client.
#[derive(Debug, Clone)]
pub struct HttpSttClient {
    provider_name: &'static str,
    endpoint: String,
    model: String,
    api_key: String,
    client: reqwest::Client,
}

impl HttpSttClient {
    #[must_use]
    pub fn new(
        provider_name: &'static str,
        endpoint: String,
        model: String,
        api_key: String,
    ) -> Self {
        Self {
            provider_name,
            endpoint,
            model,
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SttClient for HttpSttClient {
    fn provider_name(&self) -> &'static str {
        self.provider_name
    }

    async fn transcribe(&self, audio: &CapturedAudio) -> Result<String, AppError> {
        debug!(
            provider = self.provider_name,
            path = %audio.path,
            duration_ms = audio.duration_ms,
            "uploading captured audio for transcription"
        );
        let bytes = fs::read(audio.path.as_std_path())
            .await
            .map_err(|error| AppError::io("failed to read captured audio", error))?;
        let filename = audio.path.file_name().unwrap_or("capture.wav").to_string();
        let mime = match audio.path.extension() {
            Some("wav") => "audio/wav",
            Some("flac") => "audio/flac",
            _ => "application/octet-stream",
        };
        let file_part = Part::bytes(bytes)
            .file_name(filename)
            .mime_str(mime)
            .map_err(|error| {
                AppError::message(format!("failed to encode upload mime type: {error}"))
            })?;
        let form = Form::new()
            .text("model", self.model.clone())
            .part("file", file_part);

        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|error| AppError::message(format!("transcription request failed: {error}")))?;

        if !response.status().is_success() {
            return Err(AppError::message(format!(
                "transcription request returned status {}",
                response.status()
            )));
        }

        let parsed: TranscriptionResponse = response.json().await.map_err(|error| {
            AppError::message(format!("failed to decode transcription response: {error}"))
        })?;
        Ok(parsed.text)
    }
}

#[must_use]
pub fn build_http_client(
    config: &ProviderConfig,
    api_key: String,
    provider_name: &'static str,
) -> Arc<dyn SttClient> {
    let base_url = config
        .base_url
        .clone()
        .unwrap_or_else(|| match config.kind {
            ProviderKind::Groq => "https://api.groq.com/openai/v1".into(),
            ProviderKind::OpenAi => "https://api.openai.com/v1".into(),
            ProviderKind::OpenAiCompatible => unreachable!("validated elsewhere"),
        });
    let endpoint = format!("{}/audio/transcriptions", base_url.trim_end_matches('/'));
    Arc::new(HttpSttClient::new(
        provider_name,
        endpoint,
        config.model.clone(),
        api_key,
    ))
}
