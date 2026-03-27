use std::sync::Arc;

use glossa_app::ports::SttClient;
use glossa_core::ProviderConfig;

use crate::client::build_http_client;

#[must_use]
pub fn build_groq_client(config: &ProviderConfig, api_key: String) -> Arc<dyn SttClient> {
    build_http_client(config, api_key, "groq")
}
