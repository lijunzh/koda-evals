/// `part` as a whole percentage of `total`, rounded down; 0 when `total`
/// is 0.
pub fn percent(part: u64, total: u64) -> u64 {
    if total == 0 {
        return 0;
    }
    part * 100 / total
}

/// `percent` as text with a sign: `42%`.
pub fn percent_label(part: u64, total: u64) -> String {
    format!("{}%", percent(part, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_percentages() {
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent_label(1, 2), "50%");
    }
}
