/// The first letter of each word, upper-cased: "ada lovelace" → "AL".
pub fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|word| word.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}
