//! Outbound rate limit. Deliveries toward the downstream are spaced
//! evenly, so a burst from one client can't get the whole relay throttled.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Deliveries per second toward the downstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate(u32);

impl Rate {
    /// `n` deliveries per second; `n` must be positive.
    pub fn per_second(n: u32) -> Self {
        assert!(n > 0, "rate must be positive");
        Rate(n)
    }

    fn interval(self) -> Duration {
        Duration::from_secs(1) / self.0
    }
}

/// Grants slots one interval apart. Each `acquire` takes the next free
/// slot (now, if the previous slot is already past) and returns once it
/// arrives, so callers are spaced evenly at the configured rate.
pub struct Limiter {
    interval: Duration,
    next_slot: Mutex<Instant>,
}

impl Limiter {
    pub fn new(rate: Rate) -> Self {
        Self {
            interval: rate.interval(),
            next_slot: Mutex::new(Instant::now()),
        }
    }

    /// Returns once this caller's slot has arrived.
    pub async fn acquire(&self) {
        let wait = {
            let mut next = self.next_slot.lock().expect("limiter lock");
            let now = Instant::now();
            let slot = (*next).max(now);
            *next = slot + self.interval;
            slot - now
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn first_slot_is_now() {
        let limiter = Limiter::new(Rate::per_second(10));
        let started = Instant::now();
        limiter.acquire().await;
        assert!(started.elapsed() < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn slots_are_one_interval_apart() {
        let limiter = Limiter::new(Rate::per_second(100));
        let started = Instant::now();
        for _ in 0..4 {
            limiter.acquire().await;
        }
        assert!(started.elapsed() >= Duration::from_millis(30));
    }
}
