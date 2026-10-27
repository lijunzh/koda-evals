#[test]
fn numbers_chars_and_strings() {
    assert_eq!(largest(&[3, 7, 2]), Some(&7));
    assert_eq!(largest(&['a', 'z', 'm']), Some(&'z'));
    assert_eq!(largest(&["pear", "apple"]), Some(&"pear"));
}

#[test]
fn floats() {
    assert_eq!(largest(&[1.5, 0.5, 2.25]), Some(&2.25));
}

#[test]
fn empty() {
    assert_eq!(largest::<i32>(&[]), None);
}
