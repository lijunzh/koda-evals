#[test]
fn even_counts_average_the_middle_pair() {
    assert_eq!(median(&[4.0, 1.0, 3.0, 2.0]), Some(2.5));
    assert_eq!(median(&[1.0, 2.0]), Some(1.5));
}

#[test]
fn odd_and_empty_unchanged() {
    assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
    assert_eq!(median(&[7.0]), Some(7.0));
    assert_eq!(median(&[]), None);
}
