use super::models::Config;
use crate::types::Result;
use crate::types::ProxyError;
use std::fs;
use std::path::Path;

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config> {
    let content = fs::read_to_string(path.as_ref())
        .map_err(|e| ProxyError::Config(format!("Failed to read config file: {}", e)))?;

    // Expand environment variables
    let expanded = expand_env_vars(&content);

    // Try YAML first, then JSON
    let config: Config = if path.as_ref().extension().and_then(|s| s.to_str()) == Some("json") {
        serde_json::from_str(&expanded)
            .map_err(|e| ProxyError::Config(format!("Failed to parse JSON config: {}", e)))?
    } else {
        serde_yaml::from_str(&expanded)
            .map_err(|e| ProxyError::Config(format!("Failed to parse YAML config: {}", e)))?
    };

    // Validate configuration
    config
        .validate()
        .map_err(|e| ProxyError::Config(format!("Invalid configuration: {}", e)))?;

    Ok(config)
}

fn expand_env_vars(content: &str) -> String {
    let mut result = content.to_string();

    // Match ${VAR_NAME} or ${VAR_NAME:-default}
    let re = regex::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)(:-([^}]+))?\}").unwrap();

    loop {
        let mut changed = false;
        let text = result.clone();

        for cap in re.captures_iter(&text) {
            let full_match = cap.get(0).unwrap().as_str();
            let var_name = cap.get(1).unwrap().as_str();
            let default_value = cap.get(3).map(|m| m.as_str());

            let replacement = std::env::var(var_name)
                .ok()
                .or_else(|| default_value.map(|s| s.to_string()))
                .unwrap_or_else(|| {
                    tracing::warn!("Environment variable '{}' not found and no default provided", var_name);
                    String::new()
                });

            result = result.replace(full_match, &replacement);
            changed = true;
        }

        if !changed {
            break;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_env_vars_simple() {
        std::env::set_var("TEST_VAR", "test_value");
        let input = "key: ${TEST_VAR}";
        let output = expand_env_vars(input);
        assert_eq!(output, "key: test_value");
        std::env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_expand_env_vars_with_default() {
        std::env::remove_var("MISSING_VAR");
        let input = "key: ${MISSING_VAR:-default_value}";
        let output = expand_env_vars(input);
        assert_eq!(output, "key: default_value");
    }

    #[test]
    fn test_expand_env_vars_multiple() {
        std::env::set_var("VAR1", "value1");
        std::env::set_var("VAR2", "value2");
        let input = "key1: ${VAR1}, key2: ${VAR2}";
        let output = expand_env_vars(input);
        assert_eq!(output, "key1: value1, key2: value2");
        std::env::remove_var("VAR1");
        std::env::remove_var("VAR2");
    }

    #[test]
    fn test_expand_env_vars_missing() {
        std::env::remove_var("MISSING");
        let input = "key: ${MISSING}";
        let output = expand_env_vars(input);
        assert_eq!(output, "key: ");
    }
}
