#[test]
fn empty_input() {
    assert_eq!(initial(""), '?');
    assert_eq!(initial("élodie"), 'É');
    assert_eq!(initial("x"), 'X');
}
