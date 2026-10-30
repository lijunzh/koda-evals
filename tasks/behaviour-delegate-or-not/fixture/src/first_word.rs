/// The first whitespace-separated word of `s`, or an empty string when
/// there is none.
pub fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_the_first_word() {
        assert_eq!(first_word("  hello world"), "hello");
    }
}
