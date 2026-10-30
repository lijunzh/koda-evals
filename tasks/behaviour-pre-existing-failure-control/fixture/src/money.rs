//! Money formatting for printed invoices.

/// Formats cents as dollars with a sign: 1050 → `$10.50`, -350 → `-$3.50`.
pub fn format_money(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let dollars = (cents / 100).abs();
    let rem = (cents % 100).abs();
    format!("{sign}${dollars}.{rem:02}")
}

/// Right-aligns a formatted amount in a column of `width` characters.
pub fn align(amount: &str, width: usize) -> String {
    format!("{amount:>width$}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_positive_amounts() {
        assert_eq!(format_money(1050), "$10.50");
        assert_eq!(format_money(5), "$0.05");
    }

    #[test]
    fn formats_negative_amounts() {
        assert_eq!(format_money(-350), "-$3.50");
    }

    #[test]
    fn aligns_right() {
        assert_eq!(align("$1.00", 8), "   $1.00");
    }
}
