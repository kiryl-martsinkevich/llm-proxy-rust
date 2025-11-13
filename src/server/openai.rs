use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::Value;

use crate::{
    config::Config,
    proxy::ModelRouter,
    transform::{apply_header_transforms, rewrite_model_field, JsonPathTransformer, RegexTransformer},
    types::{openai::ChatCompletionRequest, ProxyError, Result},
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub router: Arc<ModelRouter>,
    pub config: Arc<Config>,
}

pub async fn chat_completions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Response> {
        let model_name = &request.model.clone();

        // Get the client and config for this model
        let client = state
            .router
            .get_client(model_name)
            .map_err(|_| ProxyError::ModelNotFound(model_name.clone()))?;

        let model_config = client.config();

        // Convert request to JSON for transformations
        let mut request_json = serde_json::to_value(&request)
            .map_err(|e| ProxyError::Transform(format!("Failed to serialize request: {}", e)))?;

        // Apply model aliasing (rewrite model field if target_model is specified)
        let target_model = model_config.get_target_model(model_name);
        if target_model != model_name {
            tracing::debug!(
                incoming_model = %model_name,
                target_model = %target_model,
                "Rewriting model field for aliasing"
            );
            request_json = rewrite_model_field(request_json, target_model)?;
        }

        // Apply request transformations
        let request_transforms = &model_config.transforms.request;
        if !request_transforms.is_empty() {
            // Apply regex transformations on the JSON string (only if there are regex transforms)
            let has_regex = request_transforms.iter().any(|t| matches!(t, crate::config::Transform::Regex { .. }));
            if has_regex {
                let regex_transformer = RegexTransformer::new(request_transforms)?;
                let json_string = serde_json::to_string(&request_json)
                    .map_err(|e| ProxyError::Transform(format!("Failed to serialize JSON: {}", e)))?;
                let transformed_string = regex_transformer.transform(&json_string);
                request_json = serde_json::from_str(&transformed_string)
                    .map_err(|e| ProxyError::Transform(format!("Failed to parse transformed JSON: {}", e)))?;
            }

            // Apply JSONPath transformations
            let jsonpath_transformer = JsonPathTransformer::new(request_transforms);
            if jsonpath_transformer.has_transforms() {
                request_json = jsonpath_transformer.transform(request_json)?;
            }
        }

        // Apply header transformations
        let mut request_headers = apply_header_transforms(&headers, &model_config.headers)?;

        // Add API key if configured
        if let Some(api_key) = client.api_key() {
            request_headers.insert(
                "authorization",
                format!("Bearer {}", api_key)
                    .parse()
                    .map_err(|e| ProxyError::Internal(format!("Invalid API key: {}", e)))?,
            );
        }

        // Convert JSON back to request body
        let request_body = serde_json::to_vec(&request_json)
            .map_err(|e| ProxyError::Transform(format!("Failed to serialize request: {}", e)))?;

        // Forward request to backend
        let response = client
            .client()
            .post(client.endpoint())
            .headers(request_headers)
            .body(request_body)
            .send()
            .await
            .map_err(|e| ProxyError::Backend(format!("Backend request failed: {}", e)))?;

        let status = response.status();
        let response_headers = response.headers().clone();
        let response_body = response
            .bytes()
            .await
            .map_err(|e| ProxyError::Backend(format!("Failed to read response: {}", e)))?;

        // Build response
        let mut response_builder = axum::response::Response::builder().status(status);

        // Copy relevant headers
        for (name, value) in response_headers.iter() {
            response_builder = response_builder.header(name, value);
        }

        let response = response_builder
            .body(axum::body::Body::from(response_body))
            .map_err(|e| ProxyError::Internal(format!("Failed to build response: {}", e)))?;

        Ok(response)
}
