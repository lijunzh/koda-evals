/// The first letter of each word, upper-cased: "ada lovelace" → "AL".
pub fn initials(name: &str) -> String {
    name.split_whitespace()
        .map(|word| word[..1].to_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_words() {
        assert_eq!(initials("ada lovelace"), "AL");
    }
}
