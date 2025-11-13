use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    pub models: HashMap<String, ModelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub include_headers: bool,
    #[serde(default = "default_true")]
    pub include_body: bool,
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            include_headers: true,
            include_body: true,
            level: "info".to_string(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub backend_type: BackendType,
    pub endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub retry: RetryConfig,
    #[serde(default = "default_true")]
    pub ssl_verify: bool,
    #[serde(default)]
    pub headers: HeaderConfig,
    #[serde(default)]
    pub transforms: TransformConfig,
}

fn default_timeout() -> u64 {
    60
}

impl ModelConfig {
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    OpenAI,
    Anthropic,
    Ollama,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: usize,
    #[serde(default = "default_backoff_ms")]
    pub backoff_ms: u64,
    #[serde(default = "default_max_backoff_ms")]
    pub max_backoff_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_ms: 1000,
            max_backoff_ms: 10000,
        }
    }
}

fn default_max_attempts() -> usize {
    3
}

fn default_backoff_ms() -> u64 {
    1000
}

fn default_max_backoff_ms() -> u64 {
    10000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderConfig {
    #[serde(default)]
    pub mode: HeaderMode,
    #[serde(default)]
    pub force: HashMap<String, String>,
    #[serde(default)]
    pub add: HashMap<String, String>,
    #[serde(default)]
    pub drop: Vec<String>,
}

impl Default for HeaderConfig {
    fn default() -> Self {
        Self {
            mode: HeaderMode::Passthrough,
            force: HashMap::new(),
            add: HashMap::new(),
            drop: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HeaderMode {
    Whitelist,
    Blacklist,
    Passthrough,
}

impl Default for HeaderMode {
    fn default() -> Self {
        HeaderMode::Passthrough
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformConfig {
    #[serde(default)]
    pub request: Vec<Transform>,
    #[serde(default)]
    pub response: Vec<Transform>,
}

impl Default for TransformConfig {
    fn default() -> Self {
        Self {
            request: Vec::new(),
            response: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Transform {
    Regex {
        pattern: String,
        replacement: String,
    },
    JsonPathDrop {
        path: String,
    },
    JsonPathAdd {
        path: String,
        value: serde_json::Value,
    },
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.models.is_empty() {
            return Err("At least one model must be configured".to_string());
        }

        for (model_name, model_config) in &self.models {
            if model_config.endpoint.is_empty() {
                return Err(format!("Model '{}' has empty endpoint", model_name));
            }

            if model_config.timeout_seconds == 0 {
                return Err(format!(
                    "Model '{}' has invalid timeout (must be > 0)",
                    model_name
                ));
            }

            if model_config.retry.max_attempts == 0 {
                return Err(format!(
                    "Model '{}' has invalid retry max_attempts (must be > 0)",
                    model_name
                ));
            }

            // Validate regex patterns
            for (idx, transform) in model_config.transforms.request.iter().enumerate() {
                if let Transform::Regex { pattern, .. } = transform {
                    regex::Regex::new(pattern)
                        .map_err(|e| format!("Invalid regex in model '{}' request transform {}: {}", model_name, idx, e))?;
                }
            }

            for (idx, transform) in model_config.transforms.response.iter().enumerate() {
                if let Transform::Regex { pattern, .. } = transform {
                    regex::Regex::new(pattern)
                        .map_err(|e| format!("Invalid regex in model '{}' response transform {}: {}", model_name, idx, e))?;
                }
            }
        }

        Ok(())
    }
}
