#[test]
fn empty_input() {
    assert_eq!(median(&[]), None);
    assert_eq!(median(&[5.0]), Some(5.0));
    assert_eq!(median(&[4.0, 1.0, 3.0, 2.0]), Some(2.5));
}
