//! Strict integer literals for the query language: an optional sign,
//! then decimal digits with no leading zeros (`0` itself is fine).

/// Parses a strict decimal literal. Returns `None` for anything that is
/// not an optional `+` or `-` followed by digits, for a leading zero on
/// a multi-digit number, and on overflow.
pub fn parse(s: &str) -> Option<i64> {
    let (negative, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    let magnitude: i64 = digits.parse().ok()?;
    if negative {
        magnitude.checked_neg()
    } else {
        Some(magnitude)
    }
}

/// Parses a comma-separated list of literals, `None` if any is invalid.
pub fn parse_list(s: &str) -> Option<Vec<i64>> {
    s.split(',').map(|item| parse(item.trim())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_and_signed() {
        assert_eq!(parse("42"), Some(42));
        assert_eq!(parse("+7"), Some(7));
        assert_eq!(parse("-12"), Some(-12));
        assert_eq!(parse("0"), Some(0));
    }

    #[test]
    fn rejects_junk_and_leading_zeros() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("-"), None);
        assert_eq!(parse("007"), None);
        assert_eq!(parse("4 2"), None);
        assert_eq!(parse("99999999999999999999"), None);
    }

    #[test]
    fn lists() {
        assert_eq!(parse_list("1, -2,3"), Some(vec![1, -2, 3]));
        assert_eq!(parse_list("1,,3"), None);
    }
}
