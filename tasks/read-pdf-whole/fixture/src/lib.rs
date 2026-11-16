//! Per-device token-bucket rate limiter. The values were agreed at the
//! review; its notes are `docs/rate-limiter-review.pdf`.

/// Bucket size: how many requests may burst at once.
pub const BURST: u32 = 0; // TODO: from the review

/// Tokens added back per second.
pub const REFILL_PER_SEC: u32 = 0; // TODO: from the review

/// Requests queued before new ones are dropped with a 429.
pub const QUEUE_LIMIT: usize = 0; // TODO: from the review

/// Seconds for an empty bucket to fill again, rounded up.
pub fn refill_secs() -> u32 {
    BURST.div_ceil(REFILL_PER_SEC)
}
