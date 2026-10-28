//! A tiny `key = value` config format. Blank lines and lines starting
//! with `#` are ignored; keys and values are trimmed.

use std::collections::BTreeMap;
use std::fmt;

/// How [`Config::parse`] treats malformed lines and key case.
#[derive(Debug, Clone)]
pub struct ParseOptions {
    /// Reject a line that has no `=` (default). When false it is skipped.
    pub strict: bool,
    /// Lower-case every key while parsing (default false).
    pub lowercase_keys: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            strict: true,
            lowercase_keys: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, Default)]
pub struct Config {
    values: BTreeMap<String, String>,
}

impl Config {
    pub fn parse(text: &str, options: &ParseOptions) -> Result<Config, ParseError> {
        let mut values = BTreeMap::new();
        for (i, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                if options.strict {
                    return Err(ParseError {
                        line: i + 1,
                        message: format!("expected `key = value`, got {line:?}"),
                    });
                }
                continue;
            };
            let key = key.trim();
            if key.is_empty() {
                return Err(ParseError {
                    line: i + 1,
                    message: "empty key".into(),
                });
            }
            let key = if options.lowercase_keys {
                key.to_lowercase()
            } else {
                key.to_string()
            };
            values.insert(key, value.trim().to_string());
        }
        Ok(Config { values })
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_rejects_junk() {
        assert!(Config::parse("junk", &ParseOptions::default()).is_err());
    }

    #[test]
    fn lenient_skips_junk_and_lowercases() {
        let opts = ParseOptions {
            strict: false,
            lowercase_keys: true,
        };
        let c = Config::parse("junk\nTitle = Hi", &opts).unwrap();
        assert_eq!(c.get("title"), Some("Hi"));
    }
}
