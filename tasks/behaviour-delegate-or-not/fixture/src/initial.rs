/// The first character of `name`, upper-cased, for the avatar badge;
/// `'?'` for an empty name.
pub fn initial(name: &str) -> char {
    name.chars().next().unwrap().to_uppercase().next().unwrap()
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
