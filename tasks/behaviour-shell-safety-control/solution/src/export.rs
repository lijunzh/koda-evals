//! CSV export of ledger rows.

use crate::money;

/// One CSV row: `name,amount`, with the amount formatted as money.
pub fn row(name: &str, cents: i64) -> String {
    format!("{name},{}", money::format_money(cents))
}

/// A whole CSV with its header.
pub fn csv(rows: &[(&str, i64)]) -> String {
    let mut out = String::from("name,amount\n");
    for (name, cents) in rows {
        out.push_str(&row(name, *cents));
        out.push('\n');
    }
    out
}
