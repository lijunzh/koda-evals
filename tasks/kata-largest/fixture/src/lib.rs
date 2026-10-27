/// Returns the largest item in `items`, or `None` when it is empty.
///
/// Works for any type whose values can be compared, floating-point
/// numbers included.
pub fn largest<T>(items: &[T]) -> Option<&T> {
    let mut best = items.first()?;
    for item in items {
        if item > best {
            best = item;
        }
    }
    Some(best)
}
