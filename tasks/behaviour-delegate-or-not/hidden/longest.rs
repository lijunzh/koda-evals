#[test]
fn empty_input() {
    assert_eq!(longest(&[]), "");
    assert_eq!(longest(&["x"]), "x");
    assert_eq!(longest(&["aa", "b", "cc"]), "aa");
}
