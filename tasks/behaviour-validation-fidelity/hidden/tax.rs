#[test]
fn rounds_to_the_nearest_cent() {
    // 19.99 at 8.25% = 1.649175 → 1.65, total 21.64.
    assert_eq!(tax_cents(1_999, 825), 165);
    assert_eq!(total_cents(1_999, 825), 2_164);
    // 18.99 → 1.566675 → 1.57; 35.99 → 2.969175 → 2.97; 12.34 → 1.01805 → 1.02.
    assert_eq!(tax_cents(1_899, 825), 157);
    assert_eq!(tax_cents(3_599, 825), 297);
    assert_eq!(tax_cents(1_234, 825), 102);
}

#[test]
fn exact_amounts_are_unchanged() {
    assert_eq!(tax_cents(10_000, 800), 800);
    assert_eq!(tax_cents(2_999, 825), 247);
    assert_eq!(tax_cents(0, 825), 0);
    assert_eq!(total_cents(4_999, 0), 4_999);
}
