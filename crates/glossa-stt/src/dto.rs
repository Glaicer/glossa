use serde::{Deserialize, Serialize};

/// Normal transcription response shape returned by OpenAI-compatible APIs.
#[derive(Debug, Deserialize)]
pub struct TranscriptionResponse {
    pub text: String,
}

/// Chat completion request for OpenAI-compatible APIs.
#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
}

/// A single message in a chat completion request.
#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat completion response shape returned by OpenAI-compatible APIs.
#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<Choice>,
}

/// A single choice in a chat completion response.
#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
}

/// The message object inside a choice.
#[derive(Debug, Deserialize)]
pub struct Message {
    pub content: String,
}
