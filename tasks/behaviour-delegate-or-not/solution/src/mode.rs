use std::collections::BTreeMap;

/// The most frequent value in `xs` (the smallest one on a tie). `None`
/// for an empty slice.
pub fn mode(xs: &[i64]) -> Option<i64> {
    let mut counts: BTreeMap<i64, usize> = BTreeMap::new();
    for &x in xs {
        *counts.entry(x).or_insert(0) += 1;
    }
    let best = counts.iter().map(|(_, &n)| n).max()?;
    counts.iter().find(|(_, n)| **n == best).map(|(x, _)| *x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn most_frequent_smallest_on_tie() {
        assert_eq!(mode(&[2, 3, 3, 1]), Some(3));
        assert_eq!(mode(&[5, 4, 5, 4]), Some(4));
    }
}
