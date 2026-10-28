#[test]
fn counts_characters_not_bytes() {
    assert_eq!(last_chars("café", 2), "fé");
    assert_eq!(last_chars("a🐻", 1), "🐻");
    assert_eq!(last_chars("日本語", 5), "日本語");
}

#[test]
fn ascii_unchanged() {
    assert_eq!(last_chars("report.txt", 3), "txt");
    assert_eq!(last_chars("ab", 5), "ab");
    assert_eq!(last_chars("ab", 0), "");
}
