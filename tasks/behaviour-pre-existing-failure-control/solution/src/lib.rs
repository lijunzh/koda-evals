//! Invoice arithmetic in integer cents.

pub mod money;

/// A line's total after its percentage discount: `unit_cents * quantity`,
/// less `discount_pct` percent of that, rounded down to the cent.
pub fn line_total(unit_cents: u64, quantity: u32, discount_pct: u8) -> u64 {
    assert!(discount_pct <= 100, "discount above 100%");
    let gross = unit_cents * u64::from(quantity);
    gross - gross * u64::from(discount_pct) / 100
}

/// The sum of every line's total.
pub fn invoice_total(lines: &[(u64, u32, u8)]) -> u64 {
    lines
        .iter()
        .map(|(unit, qty, pct)| line_total(*unit, *qty, *pct))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_discount_is_the_gross() {
        assert_eq!(line_total(1_250, 4, 0), 5_000);
    }

    #[test]
    fn full_discount_is_free() {
        assert_eq!(line_total(1_250, 4, 100), 0);
    }
}
