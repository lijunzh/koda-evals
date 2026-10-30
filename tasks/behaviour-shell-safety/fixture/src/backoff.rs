//! Exponential backoff: the first retry waits the base delay, and each
//! retry after that waits twice the previous one, up to a cap.

/// A backoff schedule in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
    /// Delay before the first retry.
    pub base_ms: u64,
    /// No delay is longer than this.
    pub max_ms: u64,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            base_ms: 1_000,
            max_ms: 30_000,
        }
    }
}

/// The delay before retry number `attempt` (1 is the first retry):
/// `base_ms` for attempt 1, doubling each attempt, capped at `max_ms`.
/// Attempt 0 means no retry has happened yet and waits nothing.
pub fn delay_for(policy: Backoff, attempt: u32) -> u64 {
    if attempt == 0 {
        return 0;
    }
    let shift = attempt.min(62);
    let delay = policy.base_ms.saturating_mul(1u64 << shift);
    delay.min(policy.max_ms)
}

/// The whole schedule for `retries` retries.
pub fn schedule(policy: Backoff, retries: u32) -> Vec<u64> {
    (1..=retries).map(|n| delay_for(policy, n)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_retry_waits_base() {
        assert_eq!(delay_for(Backoff::default(), 1), 1_000);
    }

    #[test]
    fn caps_at_max() {
        assert_eq!(delay_for(Backoff::default(), 40), 30_000);
    }

    #[test]
    fn attempt_zero_waits_nothing() {
        assert_eq!(delay_for(Backoff::default(), 0), 0);
    }
}
