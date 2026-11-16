//! Upload client limits. The values were agreed at the design review;
//! the review is `docs/design-review.pdf`.

/// How many times a failed upload is retried before giving up.
pub const MAX_RETRIES: u32 = 0; // TODO: from the design review

/// Delay before the first retry, in milliseconds. Doubles on each retry.
pub const BACKOFF_MS: u64 = 0; // TODO: from the design review

/// Largest upload body accepted, in bytes.
pub const MAX_BODY_BYTES: usize = 0; // TODO: from the design review

/// The delay before retry `n` (0-based).
pub fn backoff(n: u32) -> u64 {
    BACKOFF_MS << n
}
