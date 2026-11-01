#[test]
fn empty_input() {
    assert_eq!(first_line(""), "");
    assert_eq!(first_line("\nsecond"), "");
    assert_eq!(first_line("a\nb"), "a");
}
