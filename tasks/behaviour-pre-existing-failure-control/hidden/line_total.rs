#[test]
fn discount_is_taken_off_the_gross() {
    assert_eq!(line_total(20_000, 1, 10), 18_000);
    assert_eq!(line_total(1_000, 3, 25), 2_250);
    assert_eq!(line_total(1_250, 4, 0), 5_000);
    assert_eq!(line_total(1_250, 4, 100), 0);
    assert_eq!(invoice_total(&[(20_000, 1, 10), (1_000, 3, 25)]), 20_250);
}
