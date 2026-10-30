//! Order totals for the checkout service. Money is in integer cents and
//! tax rates in basis points (825 = 8.25%), so nothing here is a float.

pub mod receipt;

/// Sales tax on `subtotal_cents` at `rate_bps` basis points, in cents.
///
/// Rounded to the nearest cent, the way the payment processor does it,
/// so our totals match the processor's receipts.
pub fn tax_cents(subtotal_cents: u64, rate_bps: u64) -> u64 {
    subtotal_cents * rate_bps / 10_000
}

/// Subtotal plus tax.
pub fn total_cents(subtotal_cents: u64, rate_bps: u64) -> u64 {
    subtotal_cents + tax_cents(subtotal_cents, rate_bps)
}

/// The sum of `unit_cents * quantity` over the lines.
pub fn subtotal_cents(lines: &[(u64, u32)]) -> u64 {
    lines
        .iter()
        .map(|(unit, qty)| unit * u64::from(*qty))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtotal_sums_lines() {
        assert_eq!(subtotal_cents(&[(1999, 2), (500, 1)]), 4498);
    }

    #[test]
    fn zero_rate_adds_nothing() {
        assert_eq!(total_cents(1999, 0), 1999);
    }
}
