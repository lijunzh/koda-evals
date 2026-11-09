/// One ledger line. Amounts are integer cents; negative is money out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// ISO date, `YYYY-MM-DD`.
    pub date: String,
    pub payee: String,
    pub amount_cents: i64,
}

impl Entry {
    pub fn new(date: &str, payee: &str, amount_cents: i64) -> Self {
        Entry { date: date.into(), payee: payee.into(), amount_cents }
    }
}

/// Cents as a decimal string with two places: -1234 -> "-12.34".
pub fn decimal(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_formats_two_places() {
        assert_eq!(decimal(0), "0.00");
        assert_eq!(decimal(5), "0.05");
        assert_eq!(decimal(-1234), "-12.34");
        assert_eq!(decimal(100_000), "1000.00");
    }
}
