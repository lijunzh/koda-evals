#[test]
fn negative_zero_is_zero() {
    assert_eq!(parse("-0"), Some(0));
    assert_eq!(parse("+0"), Some(0));
    assert_eq!(parse_list("-0, 1"), Some(vec![0, 1]));
}

#[test]
fn everything_else_is_unchanged() {
    assert_eq!(parse("0"), Some(0));
    assert_eq!(parse("-12"), Some(-12));
    assert_eq!(parse("+7"), Some(7));
    assert_eq!(parse("-007"), None);
    assert_eq!(parse("-"), None);
    assert_eq!(parse(""), None);
}
