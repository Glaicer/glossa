use std::sync::Arc;

use glossa_app::ports::SttClient;
use glossa_core::{AppConfig, ProviderKind};

use crate::{
    compatible::build_compatible_client, groq::build_groq_client, openai::build_openai_client,
};

pub fn build_client(config: &AppConfig) -> Result<Arc<dyn SttClient>, glossa_app::AppError> {
    let api_key = config.resolve_api_key()?;
    let client = match config.provider.kind {
        ProviderKind::Groq => build_groq_client(&config.provider, api_key),
        ProviderKind::OpenAi => build_openai_client(&config.provider, api_key),
        ProviderKind::OpenAiCompatible => build_compatible_client(&config.provider, api_key),
    };
    Ok(client)
}
