use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod types;
mod proxy;
mod transform;
mod logging;
mod backends;
mod server;
mod streaming;

use config::load_config;
use proxy::ModelRouter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "llm_proxy_rust=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config/config.yaml".to_string());
    tracing::info!("Loading configuration from: {}", config_path);

    let config = load_config(&config_path)?;
    tracing::info!(
        "Configuration loaded successfully with {} models",
        config.models.len()
    );

    // Create model router
    let router = Arc::new(ModelRouter::new(&config)?);
    tracing::info!("Model router initialized with models: {:?}", router.list_models());

    // Build application router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/models", get(list_models))
        // TODO: Add OpenAI endpoints
        // .route("/v1/chat/completions", post(server::openai::chat_completions))
        // TODO: Add Anthropic endpoints
        // .route("/v1/messages", post(server::anthropic::messages))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(AppState {
            router: router.clone(),
            config: Arc::new(config.clone()),
        });

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
struct AppState {
    router: Arc<ModelRouter>,
    config: Arc<config::Config>,
}

async fn health_check() -> &'static str {
    "OK"
}

async fn list_models(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::Json<serde_json::Value> {
    let models = state.router.list_models();
    axum::Json(serde_json::json!({
        "models": models,
        "count": models.len()
    }))
}
