//! Reference tables for addresses and phone numbers: ISO 3166 countries
//! with their currencies and dialling prefixes.

pub mod countries;

pub use countries::{Country, by_alpha2, by_alpha3, by_name, by_numeric};

/// Formats an international phone number as `+<prefix> <national>`, or
/// returns the national number unchanged when the country has no prefix.
pub fn international(alpha2: &str, national: &str) -> String {
    match countries::dial_code(alpha2) {
        Some(prefix) => format!("{prefix} {national}"),
        None => national.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_with_prefix() {
        assert_eq!(international("DE", "30 1234567"), "+49 30 1234567");
        assert_eq!(international("AQ", "555"), "555");
    }
}
