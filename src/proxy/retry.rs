use crate::config::RetryConfig;
use crate::types::{ProxyError, Result};
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

pub async fn retry_with_backoff<F, Fut, T>(config: &RetryConfig, mut operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut last_error = None;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;

                if !is_retryable(&e) {
                    tracing::debug!(
                        error = %e,
                        "Error is not retryable"
                    );
                    return Err(e);
                }

                if attempt >= config.max_attempts {
                    tracing::warn!(
                        attempts = attempt,
                        max_attempts = config.max_attempts,
                        "Max retry attempts exceeded"
                    );
                    return Err(last_error.unwrap_or(ProxyError::MaxRetriesExceeded(attempt)));
                }

                let delay = calculate_backoff(attempt, config);
                tracing::info!(
                    attempt = attempt,
                    delay_ms = delay.as_millis(),
                    error = %e,
                    "Retrying request after error"
                );

                last_error = Some(e);
                sleep(delay).await;
            }
        }
    }
}

fn is_retryable(error: &ProxyError) -> bool {
    match error {
        ProxyError::Timeout => true,
        ProxyError::Upstream { status, .. } => {
            // Retry on common transient errors
            matches!(
                *status,
                429 | // Too Many Requests
                500 | // Internal Server Error
                502 | // Bad Gateway
                503 | // Service Unavailable
                504   // Gateway Timeout
            )
        }
        ProxyError::Http(e) => {
            // Retry on network errors, timeouts, etc.
            e.is_timeout() || e.is_connect() || e.is_request()
        }
        _ => false,
    }
}

fn calculate_backoff(attempt: usize, config: &RetryConfig) -> Duration {
    // Exponential backoff with jitter
    let base_delay = config.backoff_ms * (2_u64.pow(attempt as u32 - 1));
    let delay = base_delay.min(config.max_backoff_ms);

    // Add jitter (±25%)
    let jitter = (delay as f64) * 0.25;
    let jitter_range = rand::random::<f64>() * jitter * 2.0 - jitter;
    let final_delay = (delay as f64 + jitter_range).max(0.0) as u64;

    Duration::from_millis(final_delay)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable_timeout() {
        assert!(is_retryable(&ProxyError::Timeout));
    }

    #[test]
    fn test_is_retryable_upstream_errors() {
        assert!(is_retryable(&ProxyError::Upstream {
            status: 429,
            message: "Too many requests".to_string()
        }));
        assert!(is_retryable(&ProxyError::Upstream {
            status: 500,
            message: "Internal error".to_string()
        }));
        assert!(is_retryable(&ProxyError::Upstream {
            status: 502,
            message: "Bad gateway".to_string()
        }));
        assert!(is_retryable(&ProxyError::Upstream {
            status: 503,
            message: "Service unavailable".to_string()
        }));
        assert!(is_retryable(&ProxyError::Upstream {
            status: 504,
            message: "Gateway timeout".to_string()
        }));
    }

    #[test]
    fn test_is_not_retryable() {
        assert!(!is_retryable(&ProxyError::InvalidRequest(
            "Bad request".to_string()
        )));
        assert!(!is_retryable(&ProxyError::ModelNotFound(
            "model-x".to_string()
        )));
        assert!(!is_retryable(&ProxyError::Upstream {
            status: 400,
            message: "Bad request".to_string()
        }));
        assert!(!is_retryable(&ProxyError::Upstream {
            status: 401,
            message: "Unauthorized".to_string()
        }));
    }

    #[test]
    fn test_calculate_backoff() {
        let config = RetryConfig {
            max_attempts: 3,
            backoff_ms: 1000,
            max_backoff_ms: 10000,
        };

        // First retry: ~1000ms
        let delay1 = calculate_backoff(1, &config);
        assert!(delay1.as_millis() >= 750 && delay1.as_millis() <= 1250);

        // Second retry: ~2000ms
        let delay2 = calculate_backoff(2, &config);
        assert!(delay2.as_millis() >= 1500 && delay2.as_millis() <= 2500);

        // Third retry: ~4000ms
        let delay3 = calculate_backoff(3, &config);
        assert!(delay3.as_millis() >= 3000 && delay3.as_millis() <= 5000);
    }

    #[test]
    fn test_calculate_backoff_respects_max() {
        let config = RetryConfig {
            max_attempts: 10,
            backoff_ms: 1000,
            max_backoff_ms: 5000,
        };

        // Large attempt number should be capped
        let delay = calculate_backoff(10, &config);
        assert!(delay.as_millis() <= 6250); // max + 25% jitter
    }

    #[tokio::test]
    async fn test_retry_succeeds_eventually() {
        let config = RetryConfig {
            max_attempts: 3,
            backoff_ms: 10,
            max_backoff_ms: 100,
        };

        let mut attempts = 0;
        let result = retry_with_backoff(&config, || {
            attempts += 1;
            async move {
                if attempts < 2 {
                    Err(ProxyError::Timeout)
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 2);
    }

    #[tokio::test]
    async fn test_retry_fails_after_max_attempts() {
        let config = RetryConfig {
            max_attempts: 2,
            backoff_ms: 10,
            max_backoff_ms: 100,
        };

        let mut attempts = 0;
        let result = retry_with_backoff(&config, || {
            attempts += 1;
            async move { Err::<(), _>(ProxyError::Timeout) }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts, 2);
    }

    #[tokio::test]
    async fn test_retry_does_not_retry_non_retryable() {
        let config = RetryConfig {
            max_attempts: 3,
            backoff_ms: 10,
            max_backoff_ms: 100,
        };

        let mut attempts = 0;
        let result = retry_with_backoff(&config, || {
            attempts += 1;
            async move { Err::<(), _>(ProxyError::InvalidRequest("bad".to_string())) }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts, 1); // Should not retry
    }
}
