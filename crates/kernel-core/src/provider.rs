use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model_config::{ModelConfig, ProviderKind};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRequest {
    pub messages: Vec<ProviderMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderResponse {
    pub assistant_message: String,
}

pub trait ProviderAdapter: Send + Sync {
    fn generate(
        &self,
        model: &ModelConfig,
        request: &ProviderRequest,
    ) -> Result<ProviderResponse, ProviderError>;
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider does not support kind: {0:?}")]
    UnsupportedProvider(ProviderKind),
    #[error("provider endpoint is required")]
    MissingEndpoint,
    #[error("http error: {0}")]
    Http(String),
    #[error("provider returned no content")]
    EmptyResponse,
}

#[derive(Debug, Clone)]
pub struct OllamaProvider {
    client: Client,
}

impl OllamaProvider {
    pub fn new(timeout_ms: u64) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| ProviderError::Http(e.to_string()))?;

        Ok(Self { client })
    }
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage<'a>>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct OllamaMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: Option<OllamaMessageResponse>,
}

#[derive(Debug, Deserialize)]
struct OllamaMessageResponse {
    content: String,
}

impl ProviderAdapter for OllamaProvider {
    fn generate(
        &self,
        model: &ModelConfig,
        request: &ProviderRequest,
    ) -> Result<ProviderResponse, ProviderError> {
        if !matches!(model.provider, ProviderKind::Ollama | ProviderKind::Local) {
            return Err(ProviderError::UnsupportedProvider(model.provider.clone()));
        }

        let endpoint = model
            .endpoint
            .clone()
            .ok_or(ProviderError::MissingEndpoint)?;
        let url = format!("{}/api/chat", endpoint.trim_end_matches('/'));

        let payload = OllamaChatRequest {
            model: &model.model,
            messages: request
                .messages
                .iter()
                .map(|msg| OllamaMessage {
                    role: msg.role.as_str(),
                    content: msg.content.as_str(),
                })
                .collect(),
            stream: false,
            options: OllamaOptions {
                temperature: model.temperature,
            },
        };

        let response = self
            .client
            .post(url)
            .json(&payload)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| ProviderError::Http(e.to_string()))?
            .json::<OllamaChatResponse>()
            .map_err(|e| ProviderError::Http(e.to_string()))?;

        let content = response
            .message
            .map(|m| m.content)
            .filter(|c| !c.trim().is_empty())
            .ok_or(ProviderError::EmptyResponse)?;

        Ok(ProviderResponse {
            assistant_message: content,
        })
    }
}

#[derive(Debug, Clone)]
pub struct StaticProvider {
    message: String,
}

impl StaticProvider {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl ProviderAdapter for StaticProvider {
    fn generate(
        &self,
        _model: &ModelConfig,
        _request: &ProviderRequest,
    ) -> Result<ProviderResponse, ProviderError> {
        Ok(ProviderResponse {
            assistant_message: self.message.clone(),
        })
    }
}
