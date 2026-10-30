//! Receipt lines as the customer sees them.

/// Formats cents as dollars: 2164 → `$21.64`.
pub fn dollars(cents: u64) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

/// The three summary lines of a receipt.
pub fn summary(subtotal_cents: u64, rate_bps: u64) -> Vec<String> {
    let tax = crate::tax_cents(subtotal_cents, rate_bps);
    vec![
        format!("Subtotal  {}", dollars(subtotal_cents)),
        format!("Tax       {}", dollars(tax)),
        format!("Total     {}", dollars(subtotal_cents + tax)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_dollars() {
        assert_eq!(dollars(5), "$0.05");
        assert_eq!(dollars(2164), "$21.64");
    }
}
