/// The whole-number average of `xs`, rounded down; 0 for an empty slice.
pub fn average(xs: &[u64]) -> u64 {
    xs.iter().sum::<u64>() / xs.len() as u64
}

/// `average` as a label: `avg 12`.
pub fn average_label(xs: &[u64]) -> String {
    format!("avg {}", average(xs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_down() {
        assert_eq!(average(&[1, 2, 4]), 2);
        assert_eq!(average_label(&[10, 20]), "avg 15");
    }
}
