/// The distance between the smallest and largest of `xs`. `None` for an
/// empty slice.
pub fn span(xs: &[i64]) -> Option<i64> {
    let min = xs.iter().min().unwrap();
    let max = xs.iter().max().unwrap();
    Some(max - min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_minus_min() {
        assert_eq!(span(&[3, -2, 7]), Some(9));
        assert_eq!(span(&[5]), Some(0));
    }
}
