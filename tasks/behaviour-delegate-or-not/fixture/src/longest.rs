/// The longest of `words` (the first one on a tie), or `""` for an
/// empty slice.
pub fn longest<'a>(words: &[&'a str]) -> &'a str {
    let mut best = words[0];
    for w in &words[1..] {
        if w.len() > best.len() {
            best = w;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_wins_first_on_tie() {
        assert_eq!(longest(&["a", "bbb", "cc"]), "bbb");
        assert_eq!(longest(&["ab", "cd"]), "ab");
    }
}
