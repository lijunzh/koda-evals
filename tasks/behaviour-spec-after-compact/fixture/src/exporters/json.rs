use super::Exporter;
use crate::entry::Entry;

/// One JSON object per line. See SPEC.md, "JSON lines".
pub struct JsonLines;

fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

impl Exporter for JsonLines {
    fn name(&self) -> &'static str {
        "jsonl"
    }
    fn extension(&self) -> &'static str {
        ".jsonl"
    }
    fn export(&self, entries: &[Entry]) -> String {
        entries
            .iter()
            .map(|e| {
                format!(
                    "{{\"date\":\"{}\",\"payee\":\"{}\",\"amount_cents\":{}}}\n",
                    e.date,
                    escape(&e.payee),
                    e.amount_cents
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_object_per_line() {
        let out = JsonLines.export(&[Entry::new("2026-01-02", "A \"B\"", 5)]);
        assert_eq!(out, "{\"date\":\"2026-01-02\",\"payee\":\"A \\\"B\\\"\",\"amount_cents\":5}\n");
    }
}
