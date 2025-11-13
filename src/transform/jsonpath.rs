use crate::config::Transform;
use crate::types::{ProxyError, Result};
use serde_json::Value;

pub struct JsonPathTransformer {
    operations: Vec<JsonPathOp>,
}

enum JsonPathOp {
    Drop { path: String },
    Add { path: String, value: Value },
}

impl JsonPathTransformer {
    pub fn new(transforms: &[Transform]) -> Self {
        let mut operations = Vec::new();

        for transform in transforms {
            match transform {
                Transform::JsonPathDrop { path } => {
                    operations.push(JsonPathOp::Drop { path: path.clone() });
                }
                Transform::JsonPathAdd { path, value } => {
                    operations.push(JsonPathOp::Add {
                        path: path.clone(),
                        value: value.clone(),
                    });
                }
                _ => {}
            }
        }

        Self { operations }
    }

    pub fn transform(&self, mut json: Value) -> Result<Value> {
        for operation in &self.operations {
            match operation {
                JsonPathOp::Drop { path } => {
                    json = self.drop_path(&json, path)?;
                }
                JsonPathOp::Add { path, value } => {
                    json = self.add_path(json, path, value)?;
                }
            }
        }

        Ok(json)
    }

    fn drop_path(&self, json: &Value, path: &str) -> Result<Value> {
        // Simple JSONPath implementation
        // Supports basic paths like "$.field", "$.field.subfield", "$.array[0]"

        if path == "$" {
            // Can't drop root
            return Ok(json.clone());
        }

        let mut result = json.clone();
        let parts = Self::parse_path(path)?;

        if parts.is_empty() {
            return Ok(result);
        }

        // Navigate to parent and remove the last key
        if let Some((parent_path, last_key)) = Self::split_last_key(&parts) {
            if let Some(parent) = Self::navigate_to_parent(&mut result, &parent_path) {
                match parent {
                    Value::Object(map) => {
                        map.remove(&last_key);
                    }
                    Value::Array(arr) => {
                        if let Ok(index) = last_key.parse::<usize>() {
                            if index < arr.len() {
                                arr.remove(index);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(result)
    }

    fn add_path(&self, mut json: Value, path: &str, value: &Value) -> Result<Value> {
        if path == "$" {
            // Replace root
            return Ok(value.clone());
        }

        let parts = Self::parse_path(path)?;

        if parts.is_empty() {
            return Ok(json);
        }

        // Navigate and create path if needed
        if let Some((parent_path, last_key)) = Self::split_last_key(&parts) {
            let parent = Self::navigate_or_create(&mut json, &parent_path)?;

            // Ensure parent is the right type for the last_key
            if let Ok(index) = last_key.parse::<usize>() {
                // Last key is an array index
                if !parent.is_array() {
                    *parent = Value::Array(Vec::new());
                }
                if let Value::Array(arr) = parent {
                    // Extend array if needed
                    while arr.len() <= index {
                        arr.push(Value::Null);
                    }
                    arr[index] = value.clone();
                }
            } else {
                // Last key is an object key
                if !parent.is_object() {
                    *parent = Value::Object(serde_json::Map::new());
                }
                if let Value::Object(map) = parent {
                    map.insert(last_key.clone(), value.clone());
                }
            }
        }

        Ok(json)
    }

    fn parse_path(path: &str) -> Result<Vec<String>> {
        let path = path.strip_prefix("$.").unwrap_or(path);
        let path = path.strip_prefix("$").unwrap_or(path);

        if path.is_empty() {
            return Ok(Vec::new());
        }

        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_bracket = false;

        for ch in path.chars() {
            match ch {
                '.' if !in_bracket => {
                    if !current.is_empty() {
                        parts.push(current.clone());
                        current.clear();
                    }
                }
                '[' => {
                    if !current.is_empty() {
                        parts.push(current.clone());
                        current.clear();
                    }
                    in_bracket = true;
                }
                ']' => {
                    if in_bracket && !current.is_empty() {
                        parts.push(current.clone());
                        current.clear();
                    }
                    in_bracket = false;
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            parts.push(current);
        }

        Ok(parts)
    }

    fn split_last_key(parts: &[String]) -> Option<(Vec<String>, String)> {
        if parts.is_empty() {
            return None;
        }

        let parent_path = parts[..parts.len() - 1].to_vec();
        let last_key = parts[parts.len() - 1].clone();
        Some((parent_path, last_key))
    }

    fn navigate_to_parent<'a>(json: &'a mut Value, path: &[String]) -> Option<&'a mut Value> {
        let mut current = json;

        for key in path {
            current = match current {
                Value::Object(map) => map.get_mut(key)?,
                Value::Array(arr) => {
                    let index: usize = key.parse().ok()?;
                    arr.get_mut(index)?
                }
                _ => return None,
            };
        }

        Some(current)
    }

    fn navigate_or_create<'a>(
        json: &'a mut Value,
        path: &[String],
    ) -> Result<&'a mut Value> {
        let mut current = json;

        for key in path {
            // Try to parse as array index
            if let Ok(index) = key.parse::<usize>() {
                // Ensure current is an array
                if !current.is_array() {
                    *current = Value::Array(Vec::new());
                }

                if let Value::Array(arr) = current {
                    // Extend array if needed
                    while arr.len() <= index {
                        arr.push(Value::Null);
                    }
                    current = &mut arr[index];
                }
            } else {
                // Object key
                if !current.is_object() {
                    *current = Value::Object(serde_json::Map::new());
                }

                if let Value::Object(map) = current {
                    current = map.entry(key.clone()).or_insert(Value::Null);
                }
            }
        }

        Ok(current)
    }

    pub fn has_transforms(&self) -> bool {
        !self.operations.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_drop_simple_field() {
        let transforms = vec![Transform::JsonPathDrop {
            path: "$.password".to_string(),
        }];

        let transformer = JsonPathTransformer::new(&transforms);
        let input = json!({
            "username": "alice",
            "password": "secret",
            "email": "alice@example.com"
        });

        let output = transformer.transform(input).unwrap();
        assert_eq!(
            output,
            json!({
                "username": "alice",
                "email": "alice@example.com"
            })
        );
    }

    #[test]
    fn test_drop_nested_field() {
        let transforms = vec![Transform::JsonPathDrop {
            path: "$.user.password".to_string(),
        }];

        let transformer = JsonPathTransformer::new(&transforms);
        let input = json!({
            "user": {
                "username": "alice",
                "password": "secret"
            }
        });

        let output = transformer.transform(input).unwrap();
        assert_eq!(
            output,
            json!({
                "user": {
                    "username": "alice"
                }
            })
        );
    }

    #[test]
    fn test_add_simple_field() {
        let transforms = vec![Transform::JsonPathAdd {
            path: "$.proxy".to_string(),
            value: json!("llm-proxy"),
        }];

        let transformer = JsonPathTransformer::new(&transforms);
        let input = json!({
            "model": "gpt-4"
        });

        let output = transformer.transform(input).unwrap();
        assert_eq!(
            output,
            json!({
                "model": "gpt-4",
                "proxy": "llm-proxy"
            })
        );
    }

    #[test]
    fn test_add_nested_field() {
        let transforms = vec![Transform::JsonPathAdd {
            path: "$.metadata.proxy".to_string(),
            value: json!("llm-proxy"),
        }];

        let transformer = JsonPathTransformer::new(&transforms);
        let input = json!({
            "model": "gpt-4",
            "metadata": {}
        });

        let output = transformer.transform(input).unwrap();
        assert_eq!(
            output,
            json!({
                "model": "gpt-4",
                "metadata": {
                    "proxy": "llm-proxy"
                }
            })
        );
    }

    #[test]
    fn test_add_creates_path() {
        let transforms = vec![Transform::JsonPathAdd {
            path: "$.metadata.proxy.name".to_string(),
            value: json!("llm-proxy"),
        }];

        let transformer = JsonPathTransformer::new(&transforms);
        let input = json!({
            "model": "gpt-4"
        });

        let output = transformer.transform(input).unwrap();
        assert_eq!(
            output,
            json!({
                "model": "gpt-4",
                "metadata": {
                    "proxy": {
                        "name": "llm-proxy"
                    }
                }
            })
        );
    }

    #[test]
    fn test_array_access() {
        let transforms = vec![Transform::JsonPathDrop {
            path: "$.messages[1]".to_string(),
        }];

        let transformer = JsonPathTransformer::new(&transforms);
        let input = json!({
            "messages": ["msg1", "msg2", "msg3"]
        });

        let output = transformer.transform(input).unwrap();
        assert_eq!(
            output,
            json!({
                "messages": ["msg1", "msg3"]
            })
        );
    }

    #[test]
    fn test_multiple_operations() {
        let transforms = vec![
            Transform::JsonPathDrop {
                path: "$.password".to_string(),
            },
            Transform::JsonPathAdd {
                path: "$.proxy".to_string(),
                value: json!("llm-proxy"),
            },
        ];

        let transformer = JsonPathTransformer::new(&transforms);
        let input = json!({
            "username": "alice",
            "password": "secret"
        });

        let output = transformer.transform(input).unwrap();
        assert_eq!(
            output,
            json!({
                "username": "alice",
                "proxy": "llm-proxy"
            })
        );
    }

    #[test]
    fn test_parse_path() {
        assert_eq!(
            JsonPathTransformer::parse_path("$.field").unwrap(),
            vec!["field"]
        );
        assert_eq!(
            JsonPathTransformer::parse_path("$.field.subfield").unwrap(),
            vec!["field", "subfield"]
        );
        assert_eq!(
            JsonPathTransformer::parse_path("$.array[0]").unwrap(),
            vec!["array", "0"]
        );
        assert_eq!(
            JsonPathTransformer::parse_path("$.a.b[2].c").unwrap(),
            vec!["a", "b", "2", "c"]
        );
    }
}
