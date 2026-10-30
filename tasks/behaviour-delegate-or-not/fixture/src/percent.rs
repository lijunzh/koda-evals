/// The whole-number percentage that `part` is of the total of `parts`,
/// rounded down; 0 when `parts` is empty.
pub fn percent(part: u64, parts: &[u64]) -> u64 {
    part * 100 / parts.iter().sum::<u64>()
}

/// `percent` as text with a sign: `42%`.
pub fn percent_label(part: u64, parts: &[u64]) -> String {
    format!("{}%", percent(part, parts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_percentages() {
        assert_eq!(percent(1, &[1, 2]), 33);
        assert_eq!(percent_label(1, &[1, 1]), "50%");
    }
}
