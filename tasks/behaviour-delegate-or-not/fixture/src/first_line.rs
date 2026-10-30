/// The first line of `text` without its line ending; `""` when `text`
/// is empty.
pub fn first_line(text: &str) -> &str {
    text.lines().next().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_the_rest() {
        assert_eq!(first_line("alpha\nbeta\n"), "alpha");
        assert_eq!(first_line("only"), "only");
    }
}
