use crate::config::LoggingConfig;
use chrono::{DateTime, Utc};
use http::header::HeaderMap;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Serialize)]
pub struct RequestLog {
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub path: String,
    pub model: Option<String>,
    pub backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub status_code: u16,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpstreamRequestLog {
    pub timestamp: DateTime<Utc>,
    pub model: String,
    pub backend: String,
    pub endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpstreamResponseLog {
    pub timestamp: DateTime<Utc>,
    pub model: String,
    pub backend: String,
    pub status_code: u16,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub struct RequestLogger {
    config: LoggingConfig,
    start_time: Instant,
}

impl RequestLogger {
    pub fn new(config: LoggingConfig) -> Self {
        Self {
            config,
            start_time: Instant::now(),
        }
    }

    pub fn log_request(
        &self,
        method: &str,
        path: &str,
        headers: &HeaderMap,
        body: Option<&str>,
    ) {
        if !self.config.enabled {
            return;
        }

        let headers_map = if self.config.include_headers {
            Some(Self::headers_to_map(headers))
        } else {
            None
        };

        let body_str = if self.config.include_body {
            body.map(|s| s.to_string())
        } else {
            None
        };

        tracing::info!(
            method = method,
            path = path,
            headers = ?headers_map,
            body = ?body_str,
            "Incoming request"
        );
    }

    pub fn log_upstream_request(
        &self,
        model: &str,
        backend: &str,
        endpoint: &str,
        headers: &HeaderMap,
        body: Option<&str>,
    ) {
        if !self.config.enabled {
            return;
        }

        let log = UpstreamRequestLog {
            timestamp: Utc::now(),
            model: model.to_string(),
            backend: backend.to_string(),
            endpoint: endpoint.to_string(),
            headers: if self.config.include_headers {
                Some(Self::headers_to_map(headers))
            } else {
                None
            },
            body: if self.config.include_body {
                body.map(|s| s.to_string())
            } else {
                None
            },
        };

        tracing::info!(
            log = ?log,
            "Upstream request"
        );
    }

    pub fn log_upstream_response(
        &self,
        model: &str,
        backend: &str,
        status_code: u16,
        headers: &HeaderMap,
        body: Option<&str>,
        error: Option<&str>,
    ) {
        if !self.config.enabled {
            return;
        }

        let duration_ms = self.start_time.elapsed().as_millis() as u64;

        let log = UpstreamResponseLog {
            timestamp: Utc::now(),
            model: model.to_string(),
            backend: backend.to_string(),
            status_code,
            duration_ms,
            headers: if self.config.include_headers {
                Some(Self::headers_to_map(headers))
            } else {
                None
            },
            body: if self.config.include_body {
                body.map(|s| s.to_string())
            } else {
                None
            },
            error: error.map(|s| s.to_string()),
        };

        tracing::info!(
            log = ?log,
            "Upstream response"
        );
    }

    pub fn log_response(
        &self,
        method: &str,
        path: &str,
        model: Option<&str>,
        backend: Option<&str>,
        status_code: u16,
        error: Option<&str>,
    ) {
        if !self.config.enabled {
            return;
        }

        let duration_ms = self.start_time.elapsed().as_millis() as u64;

        let log = RequestLog {
            timestamp: Utc::now(),
            method: method.to_string(),
            path: path.to_string(),
            model: model.map(|s| s.to_string()),
            backend: backend.map(|s| s.to_string()),
            headers: None,
            body: None,
            status_code,
            duration_ms,
            error: error.map(|s| s.to_string()),
        };

        if status_code >= 500 {
            tracing::error!(log = ?log, "Request completed");
        } else if status_code >= 400 {
            tracing::warn!(log = ?log, "Request completed");
        } else {
            tracing::info!(log = ?log, "Request completed");
        }
    }

    fn headers_to_map(headers: &HeaderMap) -> HashMap<String, String> {
        headers
            .iter()
            .map(|(name, value)| {
                let key = name.to_string();
                let val = value.to_str().unwrap_or("<invalid>").to_string();

                // Redact sensitive headers
                let val = if Self::is_sensitive_header(&key) {
                    "[REDACTED]".to_string()
                } else {
                    val
                };

                (key, val)
            })
            .collect()
    }

    fn is_sensitive_header(name: &str) -> bool {
        let lower = name.to_lowercase();
        lower.contains("authorization")
            || lower.contains("api-key")
            || lower.contains("api_key")
            || lower.contains("x-api-key")
            || lower.contains("apikey")
            || lower.contains("token")
            || lower.contains("password")
            || lower.contains("secret")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    #[test]
    fn test_is_sensitive_header() {
        assert!(RequestLogger::is_sensitive_header("Authorization"));
        assert!(RequestLogger::is_sensitive_header("X-API-Key"));
        assert!(RequestLogger::is_sensitive_header("x-api-key"));
        assert!(RequestLogger::is_sensitive_header("Bearer-Token"));
        assert!(RequestLogger::is_sensitive_header("password"));
        assert!(RequestLogger::is_sensitive_header("Secret-Key"));

        assert!(!RequestLogger::is_sensitive_header("Content-Type"));
        assert!(!RequestLogger::is_sensitive_header("User-Agent"));
        assert!(!RequestLogger::is_sensitive_header("Accept"));
    }

    #[test]
    fn test_headers_to_map_redacts_sensitive() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("authorization", HeaderValue::from_static("Bearer secret"));
        headers.insert("x-api-key", HeaderValue::from_static("sk-123456"));

        let map = RequestLogger::headers_to_map(&headers);

        assert_eq!(map.get("content-type").unwrap(), "application/json");
        assert_eq!(map.get("authorization").unwrap(), "[REDACTED]");
        assert_eq!(map.get("x-api-key").unwrap(), "[REDACTED]");
    }

    #[test]
    fn test_logger_respects_config() {
        let config = LoggingConfig {
            enabled: false,
            include_headers: true,
            include_body: true,
            level: "info".to_string(),
        };

        let logger = RequestLogger::new(config);
        let headers = HeaderMap::new();

        // Should not panic even when logging is disabled
        logger.log_request("GET", "/test", &headers, None);
    }

    #[test]
    fn test_logger_includes_headers_and_body() {
        let config = LoggingConfig {
            enabled: true,
            include_headers: true,
            include_body: true,
            level: "info".to_string(),
        };

        let logger = RequestLogger::new(config);
        assert!(logger.config.include_headers);
        assert!(logger.config.include_body);
    }

    #[test]
    fn test_logger_excludes_headers_and_body() {
        let config = LoggingConfig {
            enabled: true,
            include_headers: false,
            include_body: false,
            level: "info".to_string(),
        };

        let logger = RequestLogger::new(config);
        assert!(!logger.config.include_headers);
        assert!(!logger.config.include_body);
    }
}
