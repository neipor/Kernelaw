use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderKind {
    OpenAI,
    Anthropic,
    Ollama,
    Local,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: ProviderKind,
    pub model: String,
    pub endpoint: Option<String>,
    pub api_key_env: Option<String>,
    pub temperature: f32,
    pub max_output_tokens: Option<u32>,
    pub timeout_ms: u64,
}

#[derive(Debug, Error)]
pub enum ModelConfigError {
    #[error("model must not be empty")]
    EmptyModel,
    #[error("temperature must be between 0.0 and 2.0")]
    InvalidTemperature,
    #[error("timeout must be at least 100ms")]
    InvalidTimeout,
}

impl ModelConfig {
    pub fn validate(&self) -> Result<(), ModelConfigError> {
        if self.model.trim().is_empty() {
            return Err(ModelConfigError::EmptyModel);
        }
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err(ModelConfigError::InvalidTemperature);
        }
        if self.timeout_ms < 100 {
            return Err(ModelConfigError::InvalidTimeout);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> ModelConfig {
        ModelConfig {
            provider: ProviderKind::Local,
            model: "qwen2.5:3b".to_string(),
            endpoint: Some("http://127.0.0.1:11434".to_string()),
            api_key_env: None,
            temperature: 0.2,
            max_output_tokens: Some(2048),
            timeout_ms: 15_000,
        }
    }

    #[test]
    fn validates_valid_config() {
        let cfg = valid();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn rejects_bad_temperature() {
        let mut cfg = valid();
        cfg.temperature = 3.0;
        assert!(matches!(
            cfg.validate(),
            Err(ModelConfigError::InvalidTemperature)
        ));
    }
}
