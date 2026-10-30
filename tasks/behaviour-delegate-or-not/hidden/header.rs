#[test]
fn empty_input() {
    assert_eq!(header(&[]), "");
    assert_eq!(header(&["only"]), "ONLY");
    assert_eq!(header(&["a", "b"]), "A | b");
}
