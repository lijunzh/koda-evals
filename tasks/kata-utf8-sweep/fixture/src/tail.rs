/// The last `n` characters of `s`, or all of `s` when it is shorter.
pub fn last_chars(s: &str, n: usize) -> &str {
    if s.len() <= n {
        return s;
    }
    &s[s.len() - n..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_the_end() {
        assert_eq!(last_chars("report.txt", 3), "txt");
    }
}
