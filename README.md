# LLM Proxy Router

A high-performance, flexible LLM API router written in Rust that acts as a proxy between clients and various LLM providers (OpenAI, Anthropic, Ollama). It provides protocol translation, request/response manipulation, and advanced routing capabilities.

## Features

### Core Capabilities

- **Multiple Protocol Support**: OpenAI-compliant and Anthropic-compliant HTTP endpoints
- **Flexible Routing**: Route requests to different backends based on model name
- **Streaming Support**: Both streaming (Server-Sent Events) and non-streaming responses
- **SSL Control**: Disable SSL verification for local/development endpoints
- **Timeout & Retry**: Configurable timeouts with exponential backoff retry logic

### Advanced Request/Response Manipulation

#### Header Manipulation
- **Three modes**: Whitelist, Blacklist, Passthrough
- **Force headers**: Override incoming headers with configured values
- **Add headers**: Add default headers if not present
- **Drop headers**: Remove specific headers from requests

#### Content Transformation
- **Regex-based search/replace**: Transform message content with regex patterns
- **JSONPath operations**:
  - Drop blocks at specified JSONPath expressions
  - Add/inject blocks at specified JSONPath expressions
- Applied to both requests and responses

### Observability

- **Comprehensive Logging**: Full request/response logging with configurable verbosity
- **Sensitive Data Redaction**: Automatic redaction of API keys, tokens, and passwords
- **Structured Logging**: JSON-formatted logs for production environments
- **Performance Metrics**: Request duration tracking

## Quick Start

### Prerequisites

- Rust 1.75+ (2021 edition)
- Configuration file (YAML or JSON)

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd llm-proxy-rust

# Build the project
cargo build --release

# Run the server
cargo run --release
```

### Basic Configuration

Create a `config/config.yaml` file:

```yaml
server:
  host: 0.0.0.0
  port: 8080

logging:
  enabled: true
  include_headers: true
  include_body: true
  level: info

models:
  gpt-4-turbo:
    backend_type: openai
    endpoint: https://api.openai.com/v1/chat/completions
    api_key: ${OPENAI_API_KEY}
    timeout_seconds: 60
    ssl_verify: true
```

### Running the Server

```bash
# Set your API keys
export OPENAI_API_KEY=sk-...
export ANTHROPIC_API_KEY=sk-ant-...

# Run with default config
cargo run --release

# Or specify a config file
CONFIG_PATH=/path/to/config.yaml cargo run --release
```

### Testing

```bash
# Check health
curl http://localhost:8080/health

# List available models
curl http://localhost:8080/models

# Make a request (once endpoints are implemented)
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4-turbo",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

## Configuration Reference

### Server Configuration

```yaml
server:
  host: 0.0.0.0      # Bind address
  port: 8080         # Listen port
```

### Logging Configuration

```yaml
logging:
  enabled: true              # Enable/disable logging
  include_headers: true      # Log HTTP headers
  include_body: true         # Log request/response bodies
  level: info               # Log level: debug, info, warn, error
```

### Model Configuration

Each model requires:

```yaml
models:
  model-name:
    backend_type: openai|anthropic|ollama
    endpoint: <backend-url>
    api_key: <api-key-or-env-var>
    target_model: <optional-model-name>  # For model aliasing
    timeout_seconds: 60
    retry:
      max_attempts: 3
      backoff_ms: 1000
      max_backoff_ms: 10000
    ssl_verify: true
    headers: <header-config>
    transforms: <transform-config>
```

### Model Aliasing

Model aliasing allows you to route requests for one model to a different backend model. This is useful for:

- **Cost optimization**: Route expensive model requests to cheaper alternatives
- **Local development**: Route production models to local Ollama instances
- **A/B testing**: Test different models without changing client code
- **Provider migration**: Gradually migrate from one provider to another

#### Example: Route GPT-4 to Local Ollama

```yaml
models:
  gpt-4:
    backend_type: ollama
    endpoint: http://localhost:11434/api/chat
    target_model: llama3-70b  # Incoming "gpt-4" -> backend "llama3-70b"
    timeout_seconds: 120
    ssl_verify: false
```

When a client sends a request with `"model": "gpt-4"`, the proxy will:
1. Route it to the configured Ollama backend
2. Rewrite the model field to `"model": "llama3-70b"`
3. Send the modified request to Ollama

#### Example: Route Claude to Self-Hosted Model

```yaml
models:
  claude-3-opus:
    backend_type: openai  # Self-hosted with OpenAI-compatible API
    endpoint: https://self-hosted.example.com/v1/chat/completions
    api_key: ${SELF_HOSTED_KEY}
    target_model: nous-hermes-2-mixtral-8x7b
    timeout_seconds: 90
```

#### Without Model Aliasing

If `target_model` is not specified, the incoming model name is used as-is:

```yaml
models:
  gpt-4-turbo:
    backend_type: openai
    endpoint: https://api.openai.com/v1/chat/completions
    api_key: ${OPENAI_API_KEY}
    # No target_model - incoming "gpt-4-turbo" -> backend "gpt-4-turbo"
```

### Header Manipulation

Three modes available:

#### 1. Whitelist Mode
Drop all incoming headers, use only configured ones:

```yaml
headers:
  mode: whitelist
  force:
    Content-Type: application/json
    User-Agent: LLMProxy/1.0
  add:
    X-Custom-Header: value
```

#### 2. Blacklist Mode
Keep all headers except those in drop list:

```yaml
headers:
  mode: blacklist
  drop:
    - X-Forwarded-For
    - X-Real-IP
```

#### 3. Passthrough Mode (default)
Keep all headers, apply force/add/drop rules:

```yaml
headers:
  mode: passthrough
  force:
    Authorization: Bearer ${API_KEY}
  add:
    X-Proxy-Version: 1.0.0
  drop:
    - Cookie
```

### Content Transformations

#### Regex Transformations

```yaml
transforms:
  request:
    - type: regex
      pattern: "\\b(password|secret|api[_-]?key)\\b"
      replacement: "[REDACTED]"
```

#### JSONPath Operations

Drop a block:
```yaml
transforms:
  request:
    - type: json_path_drop
      path: "$.messages[?(@.role=='system')]"
```

Add a block:
```yaml
transforms:
  request:
    - type: json_path_add
      path: "$.metadata"
      value:
        proxy: llm-router
        version: 1.0.0
```

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────┐
│                        Client                            │
└───────────────────┬─────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────┐
│              HTTP Server (Axum)                          │
│  - OpenAI endpoints (/v1/chat/completions)              │
│  - Anthropic endpoints (/v1/messages)                   │
└───────────────────┬─────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────┐
│              Request Pipeline                            │
│  1. Header Manipulation                                  │
│  2. Content Transformation (Regex)                       │
│  3. Content Transformation (JSONPath)                    │
└───────────────────┬─────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────────┐
│              Model Router                                │
│  - Route to backend based on model name                 │
│  - Inject API keys                                       │
└───────────────────┬─────────────────────────────────────┘
                    │
          ┌─────────┼─────────┐
          ▼         ▼         ▼
    ┌─────────┬─────────┬─────────┐
    │ OpenAI  │Anthropic│ Ollama  │
    │ Backend │ Backend │ Backend │
    └─────────┴─────────┴─────────┘
```

### Key Modules

- **config**: Configuration loading and validation
- **types**: OpenAI and Anthropic request/response types
- **proxy**: HTTP client, retry logic, model routing
- **transform**: Header manipulation, regex, JSONPath operations
- **logging**: Request/response logging with sensitive data redaction
- **backends**: Backend connectors for different LLM providers
- **server**: API endpoint handlers
- **streaming**: SSE streaming support

## Environment Variables

The configuration system supports environment variable substitution:

```yaml
# Syntax: ${VAR_NAME}
api_key: ${OPENAI_API_KEY}

# With default value: ${VAR_NAME:-default}
api_key: ${CUSTOM_API_KEY:-fallback-key}
```

Set variables before running:

```bash
export OPENAI_API_KEY=sk-...
export ANTHROPIC_API_KEY=sk-ant-...
export CUSTOM_API_KEY=custom-key
```

## Development Status

### ✅ Completed
- Project structure and dependencies
- Configuration system with YAML/JSON support
- Error handling and types
- HTTP client with SSL control
- Retry logic with exponential backoff
- Model-to-backend routing
- **Model aliasing** (route incoming model names to different backend models)
- Header manipulation (whitelist/blacklist/passthrough)
- Regex-based content transformation
- JSONPath operations
- Request/response logging
- Server foundation with health endpoints

### 🚧 In Progress
- OpenAI backend connector
- Anthropic backend connector
- Ollama backend connector
- OpenAI-compliant endpoints (streaming/non-streaming)
- Anthropic-compliant endpoints (streaming/non-streaming)
- SSE streaming support

### 📋 Planned
- Comprehensive integration tests
- Performance benchmarks
- Rate limiting
- Authentication/authorization
- Metrics and monitoring (Prometheus)
- Response caching
- Circuit breaker pattern
- Admin API for runtime configuration

## Testing

```bash
# Run unit tests
cargo test

# Run with logging
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Check code
cargo check

# Run clippy
cargo clippy

# Format code
cargo fmt
```

## Performance Considerations

- **Async all the way**: No blocking operations in request path
- **Connection pooling**: Reuse HTTP connections to backends
- **Regex compilation**: Patterns compiled at startup, not per request
- **Zero-copy operations**: Use `Bytes` for efficient data handling
- **Structured logging**: Minimal overhead with compile-time filtering

## Security

### Best Practices

1. **API Keys**: Store in environment variables, never in config files
2. **SSL Verification**: Only disable for local development
3. **Logging**: Be careful logging request/response bodies in production
4. **Sensitive Data**: Automatic redaction of common sensitive headers
5. **Regex Safety**: Validate patterns to prevent ReDoS attacks

### Sensitive Header Detection

The following headers are automatically redacted in logs:
- `Authorization`
- `X-API-Key`, `API-Key`, `APIKey`
- `*-Token`
- `*-Password`
- `*-Secret`

## Troubleshooting

### Server won't start

```bash
# Check configuration
cargo run -- --help

# Validate config file
cat config/config.yaml | yaml-validator

# Check port availability
lsof -i :8080
```

### SSL certificate errors

```bash
# Disable SSL verification for testing (NOT for production)
ssl_verify: false
```

### Environment variables not expanded

```bash
# Ensure variables are exported
env | grep API_KEY

# Check syntax in config
api_key: ${OPENAI_API_KEY}  # Correct
api_key: $OPENAI_API_KEY    # Wrong
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Write tests for new functionality
4. Ensure `cargo test` and `cargo clippy` pass
5. Submit a pull request

## License

[Specify your license here]

## Acknowledgments

Built with:
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [Tokio](https://tokio.rs/) - Async runtime
- [Reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [Serde](https://serde.rs/) - Serialization
- [Tracing](https://github.com/tokio-rs/tracing) - Logging

## Contact

For questions, issues, or feature requests, please open an issue on GitHub.
