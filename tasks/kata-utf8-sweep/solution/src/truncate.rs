/// Shortens `s` to at most `max_bytes` bytes, never splitting a character.
/// Returns `s` unchanged when it already fits.
pub fn truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
