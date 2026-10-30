use checkout::{receipt, subtotal_cents, tax_cents, total_cents};

#[test]
fn whole_dollar_amounts() {
    assert_eq!(tax_cents(10_000, 800), 800);
    assert_eq!(total_cents(10_000, 800), 10_800);
}

#[test]
fn no_tax_states() {
    assert_eq!(total_cents(4_999, 0), 4_999);
}

#[test]
fn an_exact_cent() {
    // 29.99 at 8.25% is 2.474175: 2.47 either way.
    assert_eq!(tax_cents(2_999, 825), 247);
}

#[test]
fn receipt_summary_lines_up() {
    let lines = receipt::summary(subtotal_cents(&[(10_000, 1)]), 800);
    assert_eq!(lines[2], "Total     $108.00");
}
