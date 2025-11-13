use crate::types::Result;
use serde_json::Value;

/// Rewrite the model field in a JSON request body
pub fn rewrite_model_field(mut json: Value, target_model: &str) -> Result<Value> {
    if let Some(obj) = json.as_object_mut() {
        if obj.contains_key("model") {
            obj.insert("model".to_string(), Value::String(target_model.to_string()));
        }
    }
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rewrite_model_field() {
        let input = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        let output = rewrite_model_field(input, "llama3-70b").unwrap();

        assert_eq!(
            output,
            json!({
                "model": "llama3-70b",
                "messages": [
                    {"role": "user", "content": "Hello"}
                ]
            })
        );
    }

    #[test]
    fn test_rewrite_preserves_other_fields() {
        let input = json!({
            "model": "gpt-4",
            "temperature": 0.7,
            "max_tokens": 100,
            "messages": []
        });

        let output = rewrite_model_field(input, "custom-model").unwrap();

        assert_eq!(output["model"], "custom-model");
        assert_eq!(output["temperature"], 0.7);
        assert_eq!(output["max_tokens"], 100);
    }

    #[test]
    fn test_rewrite_no_model_field() {
        let input = json!({
            "messages": []
        });

        let output = rewrite_model_field(input.clone(), "new-model").unwrap();

        // Should not add model field if it doesn't exist
        assert_eq!(output, input);
    }

    #[test]
    fn test_rewrite_non_object() {
        let input = json!("not an object");
        let output = rewrite_model_field(input.clone(), "new-model").unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn test_rewrite_array() {
        let input = json!([1, 2, 3]);
        let output = rewrite_model_field(input.clone(), "new-model").unwrap();
        assert_eq!(output, input);
    }
}
