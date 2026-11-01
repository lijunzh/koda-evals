/// The second element of `xs`, the runner-up in a ranked list. `None`
/// when `xs` has fewer than two elements.
pub fn second<T: Clone>(xs: &[T]) -> Option<T> {
    xs.get(1).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runner_up() {
        assert_eq!(second(&["gold", "silver", "bronze"]), Some("silver"));
        assert_eq!(second(&[1, 2]), Some(2));
    }
}
