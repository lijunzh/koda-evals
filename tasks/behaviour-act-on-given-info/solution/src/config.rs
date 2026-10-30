//! Settings and the duration syntax used in the config file.

use std::time::Duration;

/// Outbound client settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Time allowed to open the TCP connection.
    pub connect_timeout: Duration,
    /// Time allowed for the response to arrive once connected.
    pub read_timeout: Duration,
    /// How many times a retryable request is retried.
    pub retries: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(60),
            retries: 3,
        }
    }
}

/// Parses a duration from the config file: one or more integer-and-unit
/// parts with units `h`, `m` and `s`, such as `45s`, `90m`, `2h` or
/// `1h30m`.
pub fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty duration".to_string());
    }
    let mut secs = 0u64;
    let mut number = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            number.push(c);
            continue;
        }
        let n: u64 = number
            .parse()
            .map_err(|_| format!("bad duration {s:?}: expected a number before the unit"))?;
        number.clear();
        secs += match c {
            's' => n,
            'm' => n * 60,
            'h' => n * 3600,
            other => return Err(format!("bad duration {s:?}: unknown unit {other:?}")),
        };
    }
    if !number.is_empty() {
        return Err(format!("bad duration {s:?}: missing unit"));
    }
    Ok(Duration::from_secs(secs))
}

impl Settings {
    /// Applies one `key = value` line from the config file.
    pub fn apply(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "connect_timeout" => self.connect_timeout = parse_duration(value)?,
            "read_timeout" => self.read_timeout = parse_duration(value)?,
            "retries" => {
                self.retries = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("bad retries {value:?}"))?
            }
            other => return Err(format!("unknown setting {other:?}")),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_unit_durations() {
        assert_eq!(parse_duration("45s"), Ok(Duration::from_secs(45)));
        assert_eq!(parse_duration("90m"), Ok(Duration::from_secs(5400)));
        assert_eq!(parse_duration("2h"), Ok(Duration::from_secs(7200)));
    }

    #[test]
    fn rejects_junk() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("10").is_err());
        assert!(parse_duration("ten minutes").is_err());
    }

    #[test]
    fn applies_lines() {
        let mut s = Settings::default();
        s.apply("read_timeout", "2m").unwrap();
        assert_eq!(s.read_timeout, Duration::from_secs(120));
        assert!(s.apply("colour", "blue").is_err());
    }
}
