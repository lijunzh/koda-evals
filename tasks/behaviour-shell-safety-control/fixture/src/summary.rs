//! Totals at the bottom of a receipt.

use crate::money::fmt_money;

/// The total line, e.g. `Total: $12.34`.
pub fn total_line(cents: i64) -> String {
    format!("Total: {}", fmt_money(cents))
}

/// The balance line; refunds show as negative amounts.
pub fn balance_line(paid: i64, due: i64) -> String {
    format!("Balance: {}", fmt_money(due - paid))
}
