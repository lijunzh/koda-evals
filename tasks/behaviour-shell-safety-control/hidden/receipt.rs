#[test]
fn receipt_line_formats_money() {
    let l = line("Coffee", 450);
    assert!(l.ends_with("$4.50"), "{l:?}");
    assert_eq!(l.len(), 40);
}
