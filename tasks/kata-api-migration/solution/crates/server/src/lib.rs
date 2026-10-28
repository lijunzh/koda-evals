//! Server settings from config text.

use config_core::{Config, ParseOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

fn port(cfg: &Config) -> Result<u16, String> {
    let raw = cfg.get("port").unwrap_or("8080");
    raw.parse::<u16>()
        .map_err(|_| format!("port must be 0-65535, got {raw:?}"))
}

/// Settings with defaults: host 127.0.0.1, port 8080, 4 workers.
pub fn load(config_text: &str) -> Result<Settings, String> {
    let options = ParseOptions {
        strict: false,
        lowercase_keys: true,
    };
    let cfg = Config::parse(config_text, &options).map_err(|e| e.to_string())?;
    let host = cfg.get("host").unwrap_or("127.0.0.1").to_string();
    let port = port(&cfg)?;
    let workers = match cfg.get("workers") {
        Some(w) => w
            .parse::<usize>()
            .map_err(|_| format!("workers must be a number, got {w:?}"))?,
        None => 4,
    };
    Ok(Settings {
        host,
        port,
        workers,
    })
}

/// The startup line, e.g. "listening on 127.0.0.1:8080 (4 workers)".
pub fn banner(config_text: &str) -> String {
    match load(config_text) {
        Ok(s) => format!("listening on {}:{} ({} workers)", s.host, s.port, s.workers),
        Err(e) => format!("bad config: {e}"),
    }
}
