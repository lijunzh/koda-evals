#[test]
fn empty_input() {
    assert_eq!(last_word(""), "");
    assert_eq!(last_word("   "), "");
    assert_eq!(last_word("one two"), "two");
}
