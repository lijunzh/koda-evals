#[test]
fn never_splits_a_character() {
    assert_eq!(truncate("Élodie", 1), "");
    assert_eq!(truncate("Élodie", 2), "É");
    assert_eq!(truncate("ab🐻cd", 4), "ab");
    assert_eq!(truncate("ab🐻cd", 6), "ab🐻");
}

#[test]
fn ascii_still_cut_at_the_byte_limit() {
    assert_eq!(truncate("abcdef", 3), "abc");
    assert_eq!(truncate("abc", 3), "abc");
}
