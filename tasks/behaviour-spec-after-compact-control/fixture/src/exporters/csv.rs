use super::Exporter;
use crate::entry::{Entry, decimal};

/// RFC 4180 CSV. See SPEC.md, "CSV".
pub struct Csv;

fn field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

impl Exporter for Csv {
    fn name(&self) -> &'static str {
        "csv"
    }
    fn extension(&self) -> &'static str {
        ".csv"
    }
    fn export(&self, entries: &[Entry]) -> String {
        let mut out = String::from("date,payee,amount\r\n");
        for e in entries {
            out.push_str(&format!("{},{},{}\r\n", e.date, field(&e.payee), decimal(e.amount_cents)));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_a_payee_with_a_comma() {
        let out = Csv.export(&[Entry::new("2026-01-02", "Smith, J", -1250)]);
        assert_eq!(out, "date,payee,amount\r\n2026-01-02,\"Smith, J\",-12.50\r\n");
    }
}
