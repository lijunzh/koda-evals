use super::Exporter;
use crate::entry::Entry;

/// Tab-separated, escaped instead of quoted. See SPEC.md, "TSV".
pub struct Tsv;

fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

impl Exporter for Tsv {
    fn name(&self) -> &'static str {
        "tsv"
    }
    fn extension(&self) -> &'static str {
        ".tab"
    }
    fn export(&self, entries: &[Entry]) -> String {
        let mut out = String::from("DATE\tPAYEE\tAMOUNT\n");
        for e in entries {
            out.push_str(&format!("{}\t{}\t{}\n", e.date, escape(&e.payee), e.amount_cents));
        }
        out
    }
}
