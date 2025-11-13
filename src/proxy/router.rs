use crate::config::{Config, ModelConfig};
use crate::proxy::ProxyClient;
use crate::types::{ProxyError, Result};
use std::collections::HashMap;
use std::sync::Arc;

pub struct ModelRouter {
    clients: HashMap<String, Arc<ProxyClient>>,
}

impl ModelRouter {
    pub fn new(config: &Config) -> Result<Self> {
        let mut clients = HashMap::new();

        for (model_name, model_config) in &config.models {
            let client = Arc::new(ProxyClient::new(Arc::new(model_config.clone()))?);
            clients.insert(model_name.clone(), client);

            let target = model_config.target_model.as_deref().unwrap_or("(same)");
            tracing::info!(
                model = %model_name,
                target_model = %target,
                backend = ?model_config.backend_type,
                endpoint = %model_config.endpoint,
                ssl_verify = model_config.ssl_verify,
                "Registered model route"
            );
        }

        Ok(Self { clients })
    }

    pub fn get_client(&self, model: &str) -> Result<Arc<ProxyClient>> {
        self.clients
            .get(model)
            .cloned()
            .ok_or_else(|| ProxyError::ModelNotFound(model.to_string()))
    }

    pub fn get_config(&self, model: &str) -> Result<&ModelConfig> {
        let client = self.clients.get(model)
            .ok_or_else(|| ProxyError::ModelNotFound(model.to_string()))?;
        Ok(client.config())
    }

    pub fn list_models(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }

    pub fn has_model(&self, model: &str) -> bool {
        self.clients.contains_key(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BackendType, HeaderConfig, RetryConfig, ServerConfig, LoggingConfig, TransformConfig};

    fn create_test_config() -> Config {
        let mut models = HashMap::new();
        models.insert(
            "gpt-4".to_string(),
            ModelConfig {
                backend_type: BackendType::OpenAI,
                endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
                api_key: Some("test-key-1".to_string()),
                target_model: None,
                timeout_seconds: 60,
                retry: RetryConfig::default(),
                ssl_verify: true,
                headers: HeaderConfig::default(),
                transforms: TransformConfig::default(),
            },
        );
        models.insert(
            "claude-3".to_string(),
            ModelConfig {
                backend_type: BackendType::Anthropic,
                endpoint: "https://api.anthropic.com/v1/messages".to_string(),
                api_key: Some("test-key-2".to_string()),
                target_model: None,
                timeout_seconds: 60,
                retry: RetryConfig::default(),
                ssl_verify: true,
                headers: HeaderConfig::default(),
                transforms: TransformConfig::default(),
            },
        );

        Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            logging: LoggingConfig::default(),
            models,
        }
    }

    #[test]
    fn test_router_creation() {
        let config = create_test_config();
        let router = ModelRouter::new(&config);
        assert!(router.is_ok());
    }

    #[test]
    fn test_get_client_exists() {
        let config = create_test_config();
        let router = ModelRouter::new(&config).unwrap();
        let client = router.get_client("gpt-4");
        assert!(client.is_ok());
    }

    #[test]
    fn test_get_client_not_found() {
        let config = create_test_config();
        let router = ModelRouter::new(&config).unwrap();
        let client = router.get_client("unknown-model");
        assert!(client.is_err());
        match client {
            Err(ProxyError::ModelNotFound(model)) => {
                assert_eq!(model, "unknown-model");
            }
            _ => panic!("Expected ModelNotFound error"),
        }
    }

    #[test]
    fn test_list_models() {
        let config = create_test_config();
        let router = ModelRouter::new(&config).unwrap();
        let mut models = router.list_models();
        models.sort();
        assert_eq!(models, vec!["claude-3", "gpt-4"]);
    }

    #[test]
    fn test_has_model() {
        let config = create_test_config();
        let router = ModelRouter::new(&config).unwrap();
        assert!(router.has_model("gpt-4"));
        assert!(router.has_model("claude-3"));
        assert!(!router.has_model("unknown"));
    }

    #[test]
    fn test_target_model_aliasing() {
        let mut models = HashMap::new();

        // Model alias: gpt-4 routes to llama3-70b on Ollama
        models.insert(
            "gpt-4".to_string(),
            ModelConfig {
                backend_type: BackendType::Ollama,
                endpoint: "http://localhost:11434/api/generate".to_string(),
                api_key: None,
                target_model: Some("llama3-70b".to_string()),
                timeout_seconds: 60,
                retry: RetryConfig::default(),
                ssl_verify: false,
                headers: HeaderConfig::default(),
                transforms: TransformConfig::default(),
            },
        );

        let config = Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            logging: LoggingConfig::default(),
            models,
        };

        let router = ModelRouter::new(&config).unwrap();
        let model_config = router.get_config("gpt-4").unwrap();

        // Should route gpt-4 to llama3-70b
        assert_eq!(model_config.get_target_model("gpt-4"), "llama3-70b");
    }

    #[test]
    fn test_no_target_model_uses_incoming() {
        let config = create_test_config();
        let router = ModelRouter::new(&config).unwrap();
        let model_config = router.get_config("gpt-4").unwrap();

        // No target_model specified, should use incoming model name
        assert_eq!(model_config.get_target_model("gpt-4"), "gpt-4");
    }
}
