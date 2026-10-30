//! Core of the webhook relay. The HTTP layer (another crate) calls into
//! [`Relay`] once per request and runs those calls concurrently on one
//! tokio runtime: accept an event exactly once, deliver events to the
//! sender pool at the configured rate, answer the health endpoint.

mod journal;
mod limiter;
mod relay;

pub use limiter::Rate;
pub use relay::{Event, Health, Outcome, Relay};
