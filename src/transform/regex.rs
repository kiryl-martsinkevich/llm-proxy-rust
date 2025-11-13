use crate::config::Transform;
use crate::types::{ProxyError, Result};
use regex::Regex;
use std::collections::HashMap;

pub struct RegexTransformer {
    patterns: Vec<(Regex, String)>,
}

impl RegexTransformer {
    pub fn new(transforms: &[Transform]) -> Result<Self> {
        let mut patterns = Vec::new();

        for transform in transforms {
            if let Transform::Regex { pattern, replacement } = transform {
                let regex = Regex::new(pattern)
                    .map_err(|e| ProxyError::Regex(e))?;
                patterns.push((regex, replacement.clone()));
            }
        }

        Ok(Self { patterns })
    }

    pub fn transform(&self, input: &str) -> String {
        let mut result = input.to_string();

        for (regex, replacement) in &self.patterns {
            result = regex.replace_all(&result, replacement.as_str()).to_string();
        }

        result
    }

    pub fn has_transforms(&self) -> bool {
        !self.patterns.is_empty()
    }
}

pub struct RegexTransformCache {
    request_transformers: HashMap<String, RegexTransformer>,
    response_transformers: HashMap<String, RegexTransformer>,
}

impl RegexTransformCache {
    pub fn new() -> Self {
        Self {
            request_transformers: HashMap::new(),
            response_transformers: HashMap::new(),
        }
    }

    pub fn get_or_create_request(
        &mut self,
        model: &str,
        transforms: &[Transform],
    ) -> Result<&RegexTransformer> {
        if !self.request_transformers.contains_key(model) {
            let transformer = RegexTransformer::new(transforms)?;
            self.request_transformers.insert(model.to_string(), transformer);
        }
        Ok(self.request_transformers.get(model).unwrap())
    }

    pub fn get_or_create_response(
        &mut self,
        model: &str,
        transforms: &[Transform],
    ) -> Result<&RegexTransformer> {
        if !self.response_transformers.contains_key(model) {
            let transformer = RegexTransformer::new(transforms)?;
            self.response_transformers.insert(model.to_string(), transformer);
        }
        Ok(self.response_transformers.get(model).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_regex_transform() {
        let transforms = vec![Transform::Regex {
            pattern: r"\bpassword\b".to_string(),
            replacement: "[REDACTED]".to_string(),
        }];

        let transformer = RegexTransformer::new(&transforms).unwrap();
        let input = "My password is secret123";
        let output = transformer.transform(input);

        assert_eq!(output, "My [REDACTED] is secret123");
    }

    #[test]
    fn test_multiple_regex_transforms() {
        let transforms = vec![
            Transform::Regex {
                pattern: r"\bpassword\b".to_string(),
                replacement: "[REDACTED]".to_string(),
            },
            Transform::Regex {
                pattern: r"\bsecret\b".to_string(),
                replacement: "[HIDDEN]".to_string(),
            },
        ];

        let transformer = RegexTransformer::new(&transforms).unwrap();
        let input = "My password and secret are hidden";
        let output = transformer.transform(input);

        assert_eq!(output, "My [REDACTED] and [HIDDEN] are hidden");
    }

    #[test]
    fn test_regex_with_groups() {
        let transforms = vec![Transform::Regex {
            pattern: r"(\w+)@(\w+\.com)".to_string(),
            replacement: "[EMAIL:$2]".to_string(),
        }];

        let transformer = RegexTransformer::new(&transforms).unwrap();
        let input = "Contact: user@example.com";
        let output = transformer.transform(input);

        assert_eq!(output, "Contact: [EMAIL:example.com]");
    }

    #[test]
    fn test_no_match() {
        let transforms = vec![Transform::Regex {
            pattern: r"\bpassword\b".to_string(),
            replacement: "[REDACTED]".to_string(),
        }];

        let transformer = RegexTransformer::new(&transforms).unwrap();
        let input = "No sensitive data here";
        let output = transformer.transform(input);

        assert_eq!(output, "No sensitive data here");
    }

    #[test]
    fn test_invalid_regex() {
        let transforms = vec![Transform::Regex {
            pattern: r"[invalid".to_string(),
            replacement: "test".to_string(),
        }];

        let result = RegexTransformer::new(&transforms);
        assert!(result.is_err());
    }

    #[test]
    fn test_has_transforms() {
        let transforms = vec![Transform::Regex {
            pattern: r"test".to_string(),
            replacement: "replaced".to_string(),
        }];

        let transformer = RegexTransformer::new(&transforms).unwrap();
        assert!(transformer.has_transforms());

        let empty_transformer = RegexTransformer::new(&[]).unwrap();
        assert!(!empty_transformer.has_transforms());
    }

    #[test]
    fn test_cache() {
        let mut cache = RegexTransformCache::new();
        let transforms = vec![Transform::Regex {
            pattern: r"test".to_string(),
            replacement: "replaced".to_string(),
        }];

        let transformer1 = cache.get_or_create_request("model1", &transforms).unwrap();
        assert!(transformer1.has_transforms());

        // Should return cached transformer
        let transformer2 = cache.get_or_create_request("model1", &transforms).unwrap();
        assert!(transformer2.has_transforms());
    }
}
