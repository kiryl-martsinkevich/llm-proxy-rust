use crate::config::ModelConfig;
use crate::types::{ProxyError, Result};
use reqwest::{Client, ClientBuilder};
use std::sync::Arc;
use std::time::Duration;

pub struct ProxyClient {
    client: Client,
    config: Arc<ModelConfig>,
}

impl ProxyClient {
    pub fn new(config: Arc<ModelConfig>) -> Result<Self> {
        let mut builder = ClientBuilder::new()
            .timeout(config.timeout_duration())
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90));

        // Disable SSL verification if configured
        if !config.ssl_verify {
            tracing::warn!(
                endpoint = %config.endpoint,
                "SSL verification is disabled for this backend"
            );
            builder = builder.danger_accept_invalid_certs(true);
        }

        let client = builder
            .build()
            .map_err(|e| ProxyError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, config })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn config(&self) -> &ModelConfig {
        &self.config
    }

    pub fn endpoint(&self) -> &str {
        &self.config.endpoint
    }

    pub fn api_key(&self) -> Option<&str> {
        self.config.api_key.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BackendType, HeaderConfig, RetryConfig, TransformConfig};

    fn create_test_config(ssl_verify: bool) -> ModelConfig {
        ModelConfig {
            backend_type: BackendType::OpenAI,
            endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            api_key: Some("test-key".to_string()),
            timeout_seconds: 30,
            retry: RetryConfig::default(),
            ssl_verify,
            headers: HeaderConfig::default(),
            transforms: TransformConfig::default(),
        }
    }

    #[test]
    fn test_client_creation_with_ssl_verify() {
        let config = Arc::new(create_test_config(true));
        let client = ProxyClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_creation_without_ssl_verify() {
        let config = Arc::new(create_test_config(false));
        let client = ProxyClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_accessors() {
        let config = Arc::new(create_test_config(true));
        let client = ProxyClient::new(config).unwrap();

        assert_eq!(
            client.endpoint(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(client.api_key(), Some("test-key"));
    }
}
