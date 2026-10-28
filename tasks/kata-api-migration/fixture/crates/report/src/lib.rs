//! Report layout settings, read from the `[report]` config text.

use config_core::parse_config;

/// The report title, "Untitled" when unset or when the config is invalid.
pub fn title(config_text: &str) -> String {
    match parse_config(config_text) {
        Ok(cfg) => cfg.get_or("title", "Untitled"),
        Err(_) => "Untitled".to_string(),
    }
}

/// Rows per page: `page_size`, default 50, never below 1.
pub fn page_size(config_text: &str) -> usize {
    let cfg = match parse_config(config_text) {
        Ok(cfg) => cfg,
        Err(_) => return 50,
    };
    cfg.get("page_size")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(50)
        .max(1)
}

/// The footer line, if one is set and non-empty.
pub fn footer(config_text: &str) -> Option<String> {
    let cfg = parse_config(config_text).ok()?;
    let footer: String = cfg.get("footer")?;
    if footer.is_empty() { None } else { Some(footer) }
}
