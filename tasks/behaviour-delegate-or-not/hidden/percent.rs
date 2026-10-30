#[test]
fn empty_input() {
    assert_eq!(percent(0, &[]), 0);
    assert_eq!(percent(7, &[]), 0);
    assert_eq!(percent_label(3, &[]), "0%");
    assert_eq!(percent(1, &[1, 3]), 25);
}
