/// The first character of `name`, upper-cased, for the avatar badge;
/// `'?'` for an empty name.
pub fn initial(name: &str) -> char {
    name.chars()
        .next()
        .and_then(|c| c.to_uppercase().next())
        .unwrap_or('?')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_cases() {
        assert_eq!(initial("élodie"), 'É');
        assert_eq!(initial("bob"), 'B');
    }
}
