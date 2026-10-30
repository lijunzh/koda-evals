#[test]
fn empty_input() {
    assert_eq!(mode(&[]), None);
    assert_eq!(mode(&[7]), Some(7));
    assert_eq!(mode(&[1, 2, 2]), Some(2));
}
