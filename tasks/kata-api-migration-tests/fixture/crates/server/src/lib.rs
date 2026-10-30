//! Server settings from config text.

use config_core::{Config, parse_config};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

fn port(cfg: &Config) -> Result<u16, String> {
    let raw = cfg.get_or("port", "8080");
    raw.parse::<u16>()
        .map_err(|_| format!("port must be 0-65535, got {raw:?}"))
}

/// Settings with defaults: host 127.0.0.1, port 8080, 4 workers.
pub fn load(config_text: &str) -> Result<Settings, String> {
    let cfg = parse_config(config_text).map_err(|e| e.to_string())?;
    let host = cfg.get_or("host", "127.0.0.1");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stray_lines_and_mixed_case_keys() {
        // Ops paste these from a wiki page; stray words and capitalised keys
        // have always been accepted.
        let text = "# server\nHost = 0.0.0.0\nstray words\nPort = 9000\nWORKERS = 8\n";
        assert_eq!(
            load(text),
            Ok(Settings {
                host: "0.0.0.0".into(),
                port: 9000,
                workers: 8
            })
        );
    }

    #[test]
    fn defaults_and_errors() {
        assert_eq!(banner(""), "listening on 127.0.0.1:8080 (4 workers)");
        assert!(banner("port = 70000").starts_with("bad config: port must be"));
        assert!(banner("= x").starts_with("bad config: line 1"));
    }
}
