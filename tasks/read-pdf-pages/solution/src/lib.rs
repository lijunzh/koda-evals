//! Upload client limits. The values were agreed at the design review;
//! the review is `docs/design-review.pdf`.

/// How many times a failed upload is retried before giving up.
pub const MAX_RETRIES: u32 = 6;

/// Delay before the first retry, in milliseconds. Doubles on each retry.
pub const BACKOFF_MS: u64 = 375;

/// Largest upload body accepted, in bytes.
pub const MAX_BODY_BYTES: usize = 640 * 1024;

/// The delay before retry `n` (0-based).
pub fn backoff(n: u32) -> u64 {
    BACKOFF_MS << n
}
