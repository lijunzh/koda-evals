#[test]
fn empty_input() {
    assert_eq!(first_word(""), "");
    assert_eq!(first_word("   "), "");
    assert_eq!(first_word("one two"), "one");
}
