# LLM Proxy Router - Implementation Plan

## Project Structure
```
llm-proxy-rust/
├── Cargo.toml
├── README.md
├── CLAUDE.md
├── PLAN.md
├── config/
│   └── example-config.yaml
├── src/
│   ├── main.rs                 # Entry point, server initialization
│   ├── config/
│   │   ├── mod.rs              # Configuration module
│   │   ├── models.rs           # Configuration data structures
│   │   └── loader.rs           # YAML/JSON loading
│   ├── server/
│   │   ├── mod.rs              # Server module
│   │   ├── openai.rs           # OpenAI-compliant endpoints
│   │   ├── anthropic.rs        # Anthropic-compliant endpoints
│   │   └── middleware.rs       # Logging, error handling middleware
│   ├── proxy/
│   │   ├── mod.rs              # Proxy module
│   │   ├── router.rs           # Model-to-backend routing
│   │   ├── client.rs           # HTTP client with SSL/timeout config
│   │   └── retry.rs            # Retry logic with backoff
│   ├── transform/
│   │   ├── mod.rs              # Transformation module
│   │   ├── headers.rs          # Header manipulation
│   │   ├── regex.rs            # Regex-based content transformation
│   │   └── jsonpath.rs         # JSONPath operations
│   ├── backends/
│   │   ├── mod.rs              # Backend connectors module
│   │   ├── openai.rs           # OpenAI backend
│   │   ├── anthropic.rs        # Anthropic backend
│   │   ├── ollama.rs           # Ollama backend
│   │   └── traits.rs           # Common backend traits
│   ├── streaming/
│   │   ├── mod.rs              # Streaming module
│   │   ├── sse.rs              # Server-Sent Events handling
│   │   └── parser.rs           # Stream parsing
│   ├── logging/
│   │   ├── mod.rs              # Logging module
│   │   └── request_logger.rs   # Request/response logging
│   └── types/
│       ├── mod.rs              # Common types module
│       ├── openai.rs           # OpenAI request/response types
│       ├── anthropic.rs        # Anthropic request/response types
│       └── errors.rs           # Error types
└── tests/
    ├── integration/
    │   ├── openai_tests.rs
    │   └── anthropic_tests.rs
    └── unit/
        ├── config_tests.rs
        └── transform_tests.rs
```

## Implementation Phases

### Phase 1: Project Foundation (Priority 1)
**Goal**: Set up the basic project structure and configuration system

#### 1.1 Cargo.toml Setup
```toml
[dependencies]
axum = "0.7"
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
reqwest = { version = "0.12", features = ["json", "stream", "rustls-tls"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }
regex = "1.10"
jsonpath-rust = "0.5"
anyhow = "1.0"
thiserror = "1.0"
async-stream = "0.3"
futures = "0.3"
bytes = "1.7"
```

#### 1.2 Configuration Module
- Define configuration structs with serde
- YAML/JSON loader with environment variable substitution
- Validation logic for configuration
- Default values for optional fields

**Files**: `src/config/mod.rs`, `src/config/models.rs`, `src/config/loader.rs`

**Configuration Structure**:
```rust
struct Config {
    server: ServerConfig,
    logging: LoggingConfig,
    models: HashMap<String, ModelConfig>,
}

struct ModelConfig {
    backend_type: BackendType,
    endpoint: String,
    api_key: Option<String>,
    timeout_seconds: u64,
    retry: RetryConfig,
    ssl_verify: bool,
    headers: HeaderConfig,
    transforms: TransformConfig,
}

struct HeaderConfig {
    mode: HeaderMode,  // Whitelist, Blacklist, Passthrough
    force: HashMap<String, String>,
    add: HashMap<String, String>,
    drop: Vec<String>,
}

struct TransformConfig {
    request: Vec<Transform>,
    response: Vec<Transform>,
}

enum Transform {
    Regex { pattern: String, replacement: String },
    JsonPathDrop { path: String },
    JsonPathAdd { path: String, value: serde_json::Value },
}
```

### Phase 2: HTTP Server & Routing (Priority 1)
**Goal**: Set up Axum server with basic routing

#### 2.1 Server Setup
- Initialize Axum server
- Define route handlers
- Error handling middleware
- CORS configuration

**Files**: `src/main.rs`, `src/server/mod.rs`

#### 2.2 Routing Logic
- Model name extraction from requests
- Configuration lookup
- Backend selection
- API key injection

**Files**: `src/proxy/router.rs`

### Phase 3: Backend Connectors (Priority 1)
**Goal**: Implement HTTP clients for each backend type

#### 3.1 HTTP Client Factory
- Create reqwest client with configurable SSL verification
- Connection pooling
- Timeout configuration
- Custom TLS configuration for SSL bypass

**Files**: `src/proxy/client.rs`

#### 3.2 Backend Implementations
- OpenAI backend connector
- Anthropic backend connector
- Ollama backend connector
- Common traits for all backends

**Files**: `src/backends/*.rs`

**Key Features**:
- Non-streaming request/response handling
- Streaming support (SSE parsing and forwarding)
- Error mapping from backend to proxy errors

### Phase 4: OpenAI Protocol Implementation (Priority 1)
**Goal**: Implement OpenAI-compliant endpoints

#### 4.1 Non-Streaming Endpoint
- POST `/v1/chat/completions`
- Request parsing and validation
- Response formatting
- Error handling

**Files**: `src/server/openai.rs`, `src/types/openai.rs`

#### 4.2 Streaming Endpoint
- SSE response streaming
- Chunk parsing and forwarding
- Connection lifecycle management
- Error handling in streams

**Files**: `src/streaming/sse.rs`, `src/streaming/parser.rs`

**Request Types**:
```rust
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    stream: Option<bool>,
    // ... other OpenAI parameters
}

struct ChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
    usage: Option<Usage>,
}
```

### Phase 5: Anthropic Protocol Implementation (Priority 2)
**Goal**: Implement Anthropic-compliant endpoints

#### 5.1 Non-Streaming Endpoint
- POST `/v1/messages`
- Request parsing with Anthropic-specific fields
- Response formatting
- Error handling

**Files**: `src/server/anthropic.rs`, `src/types/anthropic.rs`

#### 5.2 Streaming Endpoint
- SSE streaming with Anthropic format
- Event parsing (message_start, content_block_delta, etc.)
- Stream forwarding

**Request Types**:
```rust
struct MessagesRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    temperature: Option<f32>,
    stream: Option<bool>,
    system: Option<String>,
    // ... other Anthropic parameters
}
```

### Phase 6: Header Manipulation (Priority 2)
**Goal**: Implement comprehensive header transformation

#### 6.1 Header Processor
- Parse header configuration
- Apply whitelist/blacklist/passthrough modes
- Force headers (override)
- Add headers (default)
- Drop headers

**Files**: `src/transform/headers.rs`

**Implementation**:
```rust
fn apply_header_transforms(
    incoming: HeaderMap,
    config: &HeaderConfig,
) -> Result<HeaderMap> {
    match config.mode {
        HeaderMode::Whitelist => {
            // Start with empty headers, add only configured ones
        }
        HeaderMode::Blacklist => {
            // Start with incoming headers, remove blocked ones
        }
        HeaderMode::Passthrough => {
            // Keep all incoming, apply force/add/drop
        }
    }
}
```

### Phase 7: Content Transformation (Priority 2)
**Goal**: Implement regex and JSONPath transformations

#### 7.1 Regex Transformer
- Compile regex patterns at config load time
- Apply search/replace on request/response bodies
- Handle multiple transformations in sequence

**Files**: `src/transform/regex.rs`

#### 7.2 JSONPath Operations
- Parse JSONPath expressions
- Drop blocks at specified paths
- Add/inject blocks at specified paths
- Preserve JSON structure

**Files**: `src/transform/jsonpath.rs`

**Implementation**:
```rust
fn apply_jsonpath_drop(
    json: &mut serde_json::Value,
    path: &str,
) -> Result<()> {
    // Use jsonpath-rust to locate and remove matching nodes
}

fn apply_jsonpath_add(
    json: &mut serde_json::Value,
    path: &str,
    value: &serde_json::Value,
) -> Result<()> {
    // Use jsonpath-rust to inject value at path
}
```

### Phase 8: Retry & Timeout Logic (Priority 2)
**Goal**: Implement resilient request handling

#### 8.1 Timeout Configuration
- Per-request timeout
- Connection timeout
- Read timeout

**Files**: `src/proxy/client.rs`

#### 8.2 Retry Logic
- Identify retryable errors (429, 500, 502, 503, 504)
- Exponential backoff
- Maximum retry attempts
- Jitter to prevent thundering herd

**Files**: `src/proxy/retry.rs`

**Implementation**:
```rust
async fn retry_with_backoff<F, Fut, T>(
    config: &RetryConfig,
    operation: F,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if is_retryable(&e) && attempt < config.max_attempts => {
                let delay = calculate_backoff(attempt, config.backoff_ms);
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### Phase 9: Logging System (Priority 2)
**Goal**: Comprehensive request/response logging

#### 9.1 Request Logger
- Log incoming requests with headers and body
- Log outgoing requests to backends
- Log responses from backends
- Log responses to clients
- Configurable verbosity

**Files**: `src/logging/request_logger.rs`

#### 9.2 Structured Logging
- Use tracing for structured logs
- JSON output for production
- Pretty output for development
- Log levels: DEBUG, INFO, WARN, ERROR

**Implementation**:
```rust
#[derive(Debug)]
struct RequestLog {
    timestamp: DateTime<Utc>,
    client_ip: IpAddr,
    method: String,
    path: String,
    model: String,
    backend: String,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
    status_code: u16,
    duration_ms: u64,
}
```

### Phase 10: Error Handling (Priority 3)
**Goal**: Robust error handling and reporting

#### 10.1 Error Types
- Define comprehensive error types
- Map backend errors to proxy errors
- Provide helpful error messages
- Include context in errors

**Files**: `src/types/errors.rs`

```rust
#[derive(Debug, thiserror::Error)]
enum ProxyError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("Transformation error: {0}")]
    Transform(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Max retries exceeded")]
    MaxRetriesExceeded,
}
```

### Phase 11: Testing (Priority 3)
**Goal**: Comprehensive test coverage

#### 11.1 Unit Tests
- Configuration parsing tests
- Header transformation tests
- Regex transformation tests
- JSONPath operation tests
- Retry logic tests

**Files**: `tests/unit/*.rs`

#### 11.2 Integration Tests
- Mock backend servers
- End-to-end request/response tests
- Streaming tests
- Error handling tests

**Files**: `tests/integration/*.rs`

### Phase 12: Documentation & Examples (Priority 3)
**Goal**: Complete documentation and examples

#### 12.1 Configuration Examples
- Multiple example configurations
- Comments explaining each option
- Common use cases

**Files**: `config/example-config.yaml`, `config/openai-only.yaml`, `config/multi-backend.yaml`

#### 12.2 README
- Quick start guide
- Configuration reference
- API documentation
- Troubleshooting guide

**Files**: `README.md`

## Implementation Order

### Week 1: Core Infrastructure
1. ✅ CLAUDE.md and PLAN.md
2. Cargo.toml setup
3. Configuration module
4. Basic server setup
5. HTTP client factory

### Week 2: OpenAI Protocol
1. OpenAI types
2. Non-streaming endpoint
3. Streaming endpoint
4. Basic routing
5. OpenAI backend connector

### Week 3: Anthropic Protocol & Additional Backends
1. Anthropic types
2. Anthropic endpoints (streaming/non-streaming)
3. Anthropic backend connector
4. Ollama backend connector
5. Generic backend connector

### Week 4: Transformations
1. Header manipulation system
2. Regex-based transformations
3. JSONPath operations
4. Transformation pipeline integration

### Week 5: Resilience & Observability
1. Timeout configuration
2. Retry logic with backoff
3. Comprehensive logging system
4. Error handling improvements

### Week 6: Testing & Polish
1. Unit tests
2. Integration tests
3. Documentation
4. Example configurations
5. README and usage guide

## Key Design Decisions

### 1. Async Runtime
- Use Tokio for async runtime (most mature, best ecosystem)
- Axum for web framework (type-safe, fast, good ergonomics)

### 2. HTTP Client
- reqwest with rustls-tls (pure Rust, no OpenSSL dependency)
- Connection pooling enabled by default
- Custom client per backend for SSL configuration

### 3. Streaming Architecture
- Use async-stream for readable streaming code
- Server-Sent Events for streaming responses
- Buffered streaming to handle backpressure

### 4. Configuration
- YAML as primary format (human-readable)
- JSON support for programmatic generation
- Environment variable substitution using `${VAR}` syntax

### 5. Error Handling
- thiserror for error type definitions
- anyhow for application errors with context
- Proper error mapping from backends to client

### 6. Logging
- tracing for structured logging
- Separate logger for request/response audit trail
- Configurable log levels per module

### 7. Security
- API keys never logged by default
- Sensitive data redaction in logs
- SSL verification enabled by default (explicit disable required)

## Performance Considerations

1. **Connection Pooling**: Reuse HTTP connections to backends
2. **Async All The Way**: No blocking operations in request path
3. **Streaming**: Stream responses without buffering entire response
4. **Regex Compilation**: Compile regex patterns at startup, not per request
5. **JSONPath Caching**: Cache compiled JSONPath expressions

## Configuration Priority

1. Environment variables (highest priority)
2. Configuration file
3. Default values (lowest priority)

## MVP Feature Set

For initial release, prioritize:
1. ✅ Project setup and documentation
2. OpenAI endpoint (non-streaming)
3. OpenAI backend connector
4. Basic routing
5. Header manipulation
6. Request/response logging
7. Basic error handling
8. Example configuration

Nice-to-have for v1.1:
- Streaming support
- Anthropic protocol
- Regex transformations
- JSONPath operations
- Retry logic
- Comprehensive tests

## Success Criteria

- Can proxy OpenAI requests to OpenAI backend
- Can proxy OpenAI requests to Anthropic backend (with protocol translation)
- Can manipulate headers per configuration
- Can log full requests/responses when enabled
- Can handle SSL verification disable
- Can retry on transient failures
- Can transform content with regex and JSONPath
- Has comprehensive test coverage
- Has clear documentation
