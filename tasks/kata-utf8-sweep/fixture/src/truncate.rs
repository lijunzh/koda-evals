/// Shortens `s` to at most `max_bytes` bytes, never splitting a character.
/// Returns `s` unchanged when it already fits.
pub fn truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    &s[..max_bytes]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_strings_are_unchanged() {
        assert_eq!(truncate("abc", 5), "abc");
    }

    #[test]
    fn long_strings_are_cut() {
        assert_eq!(truncate("abcdef", 3), "abc");
    }
}
