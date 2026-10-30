/// The last whitespace-separated word of `s`, or `""` when there is none.
pub fn last_word(s: &str) -> &str {
    s.split_whitespace().last().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_last_word() {
        assert_eq!(last_word("one two three"), "three");
        assert_eq!(last_word("  solo  "), "solo");
    }
}
