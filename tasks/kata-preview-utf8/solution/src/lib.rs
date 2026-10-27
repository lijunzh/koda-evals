/// Shortens `text` for a one-line preview.
///
/// The result keeps at most `max_bytes` bytes of `text`, cut between
/// characters, followed by `…` when anything was cut. Text that already
/// fits is returned unchanged.
pub fn preview(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_unchanged() {
        assert_eq!(preview("hi", 5), "hi");
    }

    #[test]
    fn long_text_is_cut() {
        assert_eq!(preview("hello world", 5), "hello…");
    }
}
