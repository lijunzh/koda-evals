//! Client settings for the outbound HTTP layer, read from the service's
//! config file.

pub mod config;

pub use config::{Settings, parse_duration};

/// A client built from settings; the real one wraps a connection pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    pub settings: Settings,
}

impl Client {
    pub fn from_settings(settings: Settings) -> Self {
        Self { settings }
    }

    /// The longest a single request can take: connect plus read.
    pub fn request_budget(&self) -> std::time::Duration {
        self.settings.connect_timeout + self.settings.read_timeout
    }
}
