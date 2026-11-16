//! Upload client limits. The values were agreed at the design review; the
//! whiteboard photo is `docs/limits.png`.

/// How many times a failed upload is retried before giving up.
pub const MAX_RETRIES: u32 = 5;

/// Delay before the first retry, in milliseconds. Doubles on each retry.
pub const BACKOFF_MS: u64 = 425;

/// Largest upload body accepted, in bytes.
pub const MAX_BODY_BYTES: usize = 786_432;

/// The delay before retry `n` (0-based).
pub fn backoff(n: u32) -> u64 {
    BACKOFF_MS << n
}
