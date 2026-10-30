//! Report layout settings, read from the `[report]` config text.

use config_core::{Config, ParseError, ParseOptions};

/// Parse the way config-core 1.x did: skip malformed lines, lower-case keys.
fn parse(config_text: &str) -> Result<Config, ParseError> {
    Config::parse(
        config_text,
        &ParseOptions {
            strict: false,
            lowercase_keys: true,
        },
    )
}

/// The report title, "Untitled" when unset or when the config is invalid.
pub fn title(config_text: &str) -> String {
    match parse(config_text) {
        Ok(cfg) => cfg.get("title").unwrap_or("Untitled").to_string(),
        Err(_) => "Untitled".to_string(),
    }
}

/// Rows per page: `page_size`, default 50, never below 1.
pub fn page_size(config_text: &str) -> usize {
    let cfg = match parse(config_text) {
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
    let cfg = parse(config_text).ok()?;
    let footer = cfg.get("footer")?;
    if footer.is_empty() { None } else { Some(footer.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Settings files are hand-edited; these inputs are what we see in the wild.
    const MESSY: &str = "# report settings\nTitle = Q3 Sales\nthis line is junk\nPAGE_SIZE = 0\nfooter =\n";

    #[test]
    fn junk_lines_are_skipped() {
        assert_eq!(title(MESSY), "Q3 Sales");
        assert_eq!(page_size(MESSY), 1);
    }

    #[test]
    fn keys_are_case_insensitive() {
        assert_eq!(title("TITLE = Upper"), "Upper");
        assert_eq!(page_size("Page_Size = 25"), 25);
    }

    #[test]
    fn empty_footer_is_none() {
        assert_eq!(footer(MESSY), None);
        assert_eq!(footer("footer = (c) ACME"), Some("(c) ACME".to_string()));
    }

    #[test]
    fn defaults() {
        assert_eq!(title(""), "Untitled");
        assert_eq!(page_size("page_size = abc"), 50);
        assert_eq!(title("= nameless"), "Untitled");
    }
}
