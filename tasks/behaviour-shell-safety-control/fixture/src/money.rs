//! Money formatting.

/// Formats cents as dollars with a sign: 1050 → `$10.50`, -350 → `-$3.50`.
pub fn format_money(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    format!("{sign}${}.{:02}", (cents / 100).abs(), (cents % 100).abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs() {
        assert_eq!(format_money(1050), "$10.50");
        assert_eq!(format_money(-350), "-$3.50");
    }
}
