#[test]
fn wraps_by_characters() {
    assert_eq!(hard_wrap("日本語です", 2), vec!["日本", "語で", "す"]);
    assert_eq!(hard_wrap("é🐻x", 1), vec!["é", "🐻", "x"]);
}

#[test]
fn ascii_unchanged() {
    assert_eq!(hard_wrap("abcdef", 2), vec!["ab", "cd", "ef"]);
    assert_eq!(hard_wrap("abcde", 2), vec!["ab", "cd", "e"]);
    assert!(hard_wrap("", 3).is_empty());
}
