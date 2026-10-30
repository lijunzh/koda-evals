//! Retry policy for the outbound HTTP client.

pub mod backoff;

pub use backoff::{Backoff, delay_for};

/// Whether an HTTP status is worth retrying: 408, 429 and the 5xx range.
pub fn retryable(status: u16) -> bool {
    matches!(status, 408 | 429 | 500..=599)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_statuses() {
        assert!(retryable(503));
        assert!(retryable(429));
        assert!(!retryable(404));
    }
}
