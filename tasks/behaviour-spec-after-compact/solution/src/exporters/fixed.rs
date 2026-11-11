use super::Exporter;
use crate::entry::{Entry, decimal};

/// Fixed-width lines for the line printer. See SPEC.md, "Fixed width".
pub struct Fixed;

const PAYEE: usize = 24;

fn payee(s: &str) -> String {
    let s: String = s.chars().map(|c| if c == '\t' || c == '\n' { ' ' } else { c }).collect();
    if s.chars().count() > PAYEE {
        let mut cut: String = s.chars().take(PAYEE - 1).collect();
        cut.push('…');
        cut
    } else {
        let pad = PAYEE - s.chars().count();
        format!("{s}{}", " ".repeat(pad))
    }
}

impl Exporter for Fixed {
    fn name(&self) -> &'static str {
        "fixed"
    }
    fn extension(&self) -> &'static str {
        ".txt"
    }
    fn export(&self, entries: &[Entry]) -> String {
        entries
            .iter()
            .map(|e| format!("{}  {}  {:>12}\r\n", e.date, payee(&e.payee), decimal(e.amount_cents)))
            .collect()
    }
}
