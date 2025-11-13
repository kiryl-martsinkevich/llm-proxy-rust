use crate::config::{HeaderConfig, HeaderMode};
use crate::types::{ProxyError, Result};
use http::header::{HeaderMap, HeaderName, HeaderValue};
use std::str::FromStr;

pub fn apply_header_transforms(
    incoming: &HeaderMap,
    config: &HeaderConfig,
) -> Result<HeaderMap> {
    let mut headers = match config.mode {
        HeaderMode::Whitelist => {
            // Start with empty headers, only add configured ones
            HeaderMap::new()
        }
        HeaderMode::Blacklist | HeaderMode::Passthrough => {
            // Start with all incoming headers
            incoming.clone()
        }
    };

    // Apply drop rules first (for blacklist/passthrough modes)
    if !config.drop.is_empty() {
        for header_name in &config.drop {
            let name = HeaderName::from_str(header_name)
                .map_err(|e| ProxyError::Header(format!("Invalid header name '{}': {}", header_name, e)))?;
            headers.remove(&name);
        }
    }

    // Apply add rules (only if header doesn't exist)
    for (key, value) in &config.add {
        let name = HeaderName::from_str(key)
            .map_err(|e| ProxyError::Header(format!("Invalid header name '{}': {}", key, e)))?;

        // Only add if not already present
        if !headers.contains_key(&name) {
            let val = HeaderValue::from_str(value)
                .map_err(|e| ProxyError::Header(format!("Invalid header value for '{}': {}", key, e)))?;
            headers.insert(name, val);
        }
    }

    // Apply force rules (override existing headers)
    for (key, value) in &config.force {
        let name = HeaderName::from_str(key)
            .map_err(|e| ProxyError::Header(format!("Invalid header name '{}': {}", key, e)))?;
        let val = HeaderValue::from_str(value)
            .map_err(|e| ProxyError::Header(format!("Invalid header value for '{}': {}", key, e)))?;
        headers.insert(name, val);
    }

    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("x-api-key", "secret".parse().unwrap());
        headers.insert("user-agent", "test-client".parse().unwrap());
        headers
    }

    #[test]
    fn test_passthrough_mode_no_changes() {
        let config = HeaderConfig {
            mode: HeaderMode::Passthrough,
            force: HashMap::new(),
            add: HashMap::new(),
            drop: Vec::new(),
        };

        let incoming = create_test_headers();
        let result = apply_header_transforms(&incoming, &config).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result.get("content-type").unwrap(), "application/json");
        assert_eq!(result.get("x-api-key").unwrap(), "secret");
        assert_eq!(result.get("user-agent").unwrap(), "test-client");
    }

    #[test]
    fn test_whitelist_mode() {
        let mut force = HashMap::new();
        force.insert("content-type".to_string(), "text/plain".to_string());

        let config = HeaderConfig {
            mode: HeaderMode::Whitelist,
            force,
            add: HashMap::new(),
            drop: Vec::new(),
        };

        let incoming = create_test_headers();
        let result = apply_header_transforms(&incoming, &config).unwrap();

        // Only the forced header should be present
        assert_eq!(result.len(), 1);
        assert_eq!(result.get("content-type").unwrap(), "text/plain");
        assert!(result.get("x-api-key").is_none());
        assert!(result.get("user-agent").is_none());
    }

    #[test]
    fn test_drop_headers() {
        let config = HeaderConfig {
            mode: HeaderMode::Passthrough,
            force: HashMap::new(),
            add: HashMap::new(),
            drop: vec!["x-api-key".to_string(), "user-agent".to_string()],
        };

        let incoming = create_test_headers();
        let result = apply_header_transforms(&incoming, &config).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.get("content-type").unwrap(), "application/json");
        assert!(result.get("x-api-key").is_none());
        assert!(result.get("user-agent").is_none());
    }

    #[test]
    fn test_add_headers() {
        let mut add = HashMap::new();
        add.insert("x-custom-header".to_string(), "custom-value".to_string());
        add.insert("content-type".to_string(), "should-not-override".to_string());

        let config = HeaderConfig {
            mode: HeaderMode::Passthrough,
            force: HashMap::new(),
            add,
            drop: Vec::new(),
        };

        let incoming = create_test_headers();
        let result = apply_header_transforms(&incoming, &config).unwrap();

        assert_eq!(result.len(), 4);
        assert_eq!(result.get("content-type").unwrap(), "application/json"); // Original value
        assert_eq!(result.get("x-custom-header").unwrap(), "custom-value");
    }

    #[test]
    fn test_force_headers() {
        let mut force = HashMap::new();
        force.insert("content-type".to_string(), "text/plain".to_string());
        force.insert("x-new-header".to_string(), "new-value".to_string());

        let config = HeaderConfig {
            mode: HeaderMode::Passthrough,
            force,
            add: HashMap::new(),
            drop: Vec::new(),
        };

        let incoming = create_test_headers();
        let result = apply_header_transforms(&incoming, &config).unwrap();

        assert_eq!(result.len(), 4);
        assert_eq!(result.get("content-type").unwrap(), "text/plain"); // Overridden
        assert_eq!(result.get("x-new-header").unwrap(), "new-value");
    }

    #[test]
    fn test_combined_operations() {
        let mut force = HashMap::new();
        force.insert("content-type".to_string(), "text/plain".to_string());

        let mut add = HashMap::new();
        add.insert("x-custom".to_string(), "custom".to_string());

        let config = HeaderConfig {
            mode: HeaderMode::Passthrough,
            force,
            add,
            drop: vec!["x-api-key".to_string()],
        };

        let incoming = create_test_headers();
        let result = apply_header_transforms(&incoming, &config).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result.get("content-type").unwrap(), "text/plain");
        assert_eq!(result.get("user-agent").unwrap(), "test-client");
        assert_eq!(result.get("x-custom").unwrap(), "custom");
        assert!(result.get("x-api-key").is_none());
    }

    #[test]
    fn test_blacklist_mode() {
        let config = HeaderConfig {
            mode: HeaderMode::Blacklist,
            force: HashMap::new(),
            add: HashMap::new(),
            drop: vec!["x-api-key".to_string()],
        };

        let incoming = create_test_headers();
        let result = apply_header_transforms(&incoming, &config).unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.get("content-type").is_some());
        assert!(result.get("user-agent").is_some());
        assert!(result.get("x-api-key").is_none());
    }
}
