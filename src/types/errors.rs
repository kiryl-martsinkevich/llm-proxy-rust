use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Model '{0}' not found in configuration")]
    ModelNotFound(String),

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("Upstream error: {status} - {message}")]
    Upstream { status: u16, message: String },

    #[error("Transformation error: {0}")]
    Transform(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Max retries exceeded after {0} attempts")]
    MaxRetriesExceeded(usize),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Header error: {0}")]
    Header(String),

    #[error("Streaming error: {0}")]
    Streaming(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl ProxyError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ProxyError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ProxyError::ModelNotFound(_) => StatusCode::NOT_FOUND,
            ProxyError::Backend(_) => StatusCode::BAD_GATEWAY,
            ProxyError::Upstream { status, .. } => {
                StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY)
            }
            ProxyError::Transform(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ProxyError::Timeout => StatusCode::GATEWAY_TIMEOUT,
            ProxyError::MaxRetriesExceeded(_) => StatusCode::BAD_GATEWAY,
            ProxyError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            ProxyError::Http(_) => StatusCode::BAD_GATEWAY,
            ProxyError::Json(_) => StatusCode::BAD_REQUEST,
            ProxyError::Yaml(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ProxyError::Regex(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ProxyError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ProxyError::Header(_) => StatusCode::BAD_REQUEST,
            ProxyError::Streaming(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ProxyError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_type(&self) -> &str {
        match self {
            ProxyError::Config(_) => "configuration_error",
            ProxyError::ModelNotFound(_) => "model_not_found",
            ProxyError::Backend(_) => "backend_error",
            ProxyError::Upstream { .. } => "upstream_error",
            ProxyError::Transform(_) => "transformation_error",
            ProxyError::Timeout => "timeout",
            ProxyError::MaxRetriesExceeded(_) => "max_retries_exceeded",
            ProxyError::InvalidRequest(_) => "invalid_request",
            ProxyError::Http(_) => "http_error",
            ProxyError::Json(_) => "json_error",
            ProxyError::Yaml(_) => "yaml_error",
            ProxyError::Regex(_) => "regex_error",
            ProxyError::Io(_) => "io_error",
            ProxyError::Header(_) => "header_error",
            ProxyError::Streaming(_) => "streaming_error",
            ProxyError::Internal(_) => "internal_error",
        }
    }
}

// Implement IntoResponse for ProxyError to convert errors into HTTP responses
impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_type = self.error_type();
        let message = self.to_string();

        tracing::error!(
            error_type = error_type,
            status = status.as_u16(),
            message = %message,
            "Request failed"
        );

        let body = Json(json!({
            "error": {
                "type": error_type,
                "message": message,
                "code": status.as_u16(),
            }
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, ProxyError>;
