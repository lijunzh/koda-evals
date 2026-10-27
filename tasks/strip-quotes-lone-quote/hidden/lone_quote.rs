#[test]
fn lone_quotes_are_left_alone() {
    assert_eq!(strip_quotes("'"), "'");
    assert_eq!(strip_quotes("\""), "\"");
}

#[test]
fn quoted_paths_are_still_unquoted() {
    assert_eq!(strip_quotes("'/path/to/file.png'"), "/path/to/file.png");
    assert_eq!(strip_quotes("\"/path/to/file.png\""), "/path/to/file.png");
    assert_eq!(strip_quotes("''"), "");
}
