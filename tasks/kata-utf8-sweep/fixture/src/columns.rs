/// Splits `s` into lines of at most `width` characters each, breaking
/// anywhere (not only at spaces). `width` must be at least 1.
pub fn hard_wrap(s: &str, width: usize) -> Vec<String> {
    assert!(width >= 1, "width must be at least 1");
    let mut lines = Vec::new();
    let mut start = 0;
    while start < s.len() {
        let end = (start + width).min(s.len());
        lines.push(s[start..end].to_string());
        start = end;
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_evenly() {
        assert_eq!(hard_wrap("abcdef", 2), vec!["ab", "cd", "ef"]);
    }
}
