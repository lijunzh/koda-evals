use crate::exporters::{Exporter, csv::Csv, json::JsonLines};

/// Every exporter, in the order SPEC.md's "Registry" section requires.
pub fn exporters() -> Vec<Box<dyn Exporter>> {
    vec![Box::new(Csv), Box::new(JsonLines)]
}

/// The exporter a user named, if there is one.
pub fn by_name(name: &str) -> Option<Box<dyn Exporter>> {
    exporters().into_iter().find(|e| e.name() == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_csv() {
        assert_eq!(by_name("csv").map(|e| e.extension()), Some(".csv"));
        assert!(by_name("xml").is_none());
    }
}
