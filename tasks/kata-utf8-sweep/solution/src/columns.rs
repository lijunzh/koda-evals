/// Splits `s` into lines of at most `width` characters each, breaking
/// anywhere (not only at spaces). `width` must be at least 1.
pub fn hard_wrap(s: &str, width: usize) -> Vec<String> {
    assert!(width >= 1, "width must be at least 1");
    let chars: Vec<char> = s.chars().collect();
    chars.chunks(width).map(|c| c.iter().collect()).collect()
}
