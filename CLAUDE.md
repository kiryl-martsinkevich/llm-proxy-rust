# LLM Proxy Router - Project Context

## Overview
This is a Rust-based LLM API router service that acts as a proxy between clients and various LLM providers (OpenAI, Anthropic, Ollama). It provides protocol translation, request/response manipulation, and advanced routing capabilities.

## Core Purpose
- **Protocol Translation**: Accept requests in OpenAI or Anthropic format and route them to any supported backend
- **Request Manipulation**: Transform requests through header manipulation, content regex replacement, and JSON path operations
- **Observability**: Comprehensive logging of requests/responses with configurable verbosity
- **Flexibility**: Support for SSL verification bypass, custom timeouts, and retry logic

## Architecture

### Components
1. **HTTP Server** (Axum-based)
   - OpenAI-compliant endpoints (`/v1/chat/completions`, etc.)
   - Anthropic-compliant endpoints (`/v1/messages`, etc.)
   - Handles both streaming (SSE) and non-streaming responses

2. **Configuration System**
   - YAML/JSON configuration file support
   - Model-to-backend routing rules
   - Per-model API key configuration
   - Header manipulation rules
   - Content transformation rules (regex, JSONPath)

3. **Request Pipeline**
   ```
   Client Request → Header Manipulation → Content Transformation →
   Backend Routing → Upstream Request → Response Transformation → Client Response
   ```

4. **Backend Connectors**
   - OpenAI connector (streaming/non-streaming)
   - Anthropic connector (streaming/non-streaming)
   - Ollama connector
   - Generic HTTP/HTTPS connector with SSL bypass option

5. **Logging System**
   - Request/response logging with headers
   - Configurable log levels
   - Structured logging for audit trails

## Key Features

### 1. Header Manipulation
- Drop all incoming headers and use only configured ones
- Force specific headers (override client values)
- Add default headers if not present
- Drop specific headers by name

### 2. Content Transformation
- **Regex-based search/replace**: Transform message content with regex patterns
- **JSONPath operations**:
  - Drop blocks at specified JSONPath
  - Add/inject blocks at specified JSONPath

### 3. Routing Configuration
```yaml
models:
  gpt-4:
    backend: openai
    endpoint: https://api.openai.com/v1
    api_key: sk-...

  claude-3:
    backend: anthropic
    endpoint: https://api.anthropic.com/v1
    api_key: sk-ant-...

  local-llama:
    backend: ollama
    endpoint: http://localhost:11434
```

### 4. Timeout & Retry
- Configurable per-model timeouts
- Retry on transient errors (429, 500, 502, 503, 504)
- Exponential backoff strategy
- Maximum retry attempts

### 5. SSL Verification Control
- Disable SSL verification for local/development backends
- Per-backend SSL configuration

## Technology Stack
- **Web Framework**: Axum (tokio-based async runtime)
- **HTTP Client**: reqwest (with rustls for SSL)
- **Configuration**: serde with YAML/JSON support
- **Logging**: tracing + tracing-subscriber
- **JSON Processing**: serde_json + jsonpath-rust
- **Regex**: regex crate
- **Streaming**: Server-Sent Events (SSE) for streaming responses

## Configuration Example
```yaml
server:
  host: 0.0.0.0
  port: 8080

logging:
  enabled: true
  include_headers: true
  include_body: true

models:
  gpt-4-turbo:
    backend_type: openai
    endpoint: https://api.openai.com/v1/chat/completions
    api_key: ${OPENAI_API_KEY}
    timeout_seconds: 60
    retry:
      max_attempts: 3
      backoff_ms: 1000
    ssl_verify: true
    headers:
      mode: whitelist  # drop all, then add these
      force:
        Content-Type: application/json
        User-Agent: LLMProxy/1.0
      add:
        X-Custom-Header: value
      drop:
        - X-Forwarded-For
    transforms:
      request:
        - type: regex
          pattern: "\\b(password|secret)\\b"
          replacement: "[REDACTED]"
        - type: jsonpath_drop
          path: "$.messages[?(@.role=='system')]"
        - type: jsonpath_add
          path: "$.metadata"
          value: {"proxy": "llm-router"}
```

## Development Phases
1. **Foundation**: Basic server + configuration loading
2. **Routing**: Model-to-backend routing with API keys
3. **Protocols**: OpenAI and Anthropic endpoint implementations
4. **Streaming**: SSE support for streaming responses
5. **Manipulation**: Header and content transformation
6. **Resilience**: Timeouts, retries, error handling
7. **Observability**: Comprehensive logging
8. **Testing**: Unit and integration tests

## Security Considerations
- API keys in environment variables or secure configuration
- Request/response logging should be carefully controlled in production
- SSL verification bypass should only be used in controlled environments
- Regex transformations should be validated to prevent ReDoS attacks

## Future Enhancements
- Rate limiting per model/client
- Authentication/authorization layer
- Metrics and monitoring (Prometheus)
- Response caching
- Load balancing across multiple backends
- Circuit breaker pattern
- Admin API for runtime configuration updates
