//! The profile service behind the account pages: a read-through cache in
//! front of the profile database.
//!
//! The HTTP layer (another crate) calls [`Profiles`] from many tasks at once
//! on a multi-threaded tokio runtime, and the Postgres crate implements
//! [`Store`]. Keep the public signatures of both: they are this crate's API.

mod cache;
mod service;
mod store;

pub use cache::Cache;
pub use service::{Error, Profiles};
pub use store::{BoxFuture, MemStore, Profile, Store, StoreError, UserId};
