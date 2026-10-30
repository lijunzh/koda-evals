#[test]
fn empty_input() {
    assert_eq!(average(&[]), 0);
    assert_eq!(average_label(&[]), "avg 0");
    assert_eq!(average(&[3, 4]), 3);
}
