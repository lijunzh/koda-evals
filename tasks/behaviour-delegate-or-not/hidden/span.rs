#[test]
fn empty_input() {
    assert_eq!(span(&[]), None);
    assert_eq!(span(&[-1, 1]), Some(2));
}
