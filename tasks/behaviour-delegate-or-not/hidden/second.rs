#[test]
fn empty_input() {
    assert_eq!(second::<i32>(&[]), None);
    assert_eq!(second(&[1]), None);
    assert_eq!(second(&[1, 2, 3]), Some(2));
}
