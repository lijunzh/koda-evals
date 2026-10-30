//! One receipt line per item.

use crate::money::fmt_money;

/// `<name>  <amount>` padded so the amounts line up at 40 columns.
pub fn line(name: &str, cents: i64) -> String {
    let amount = fmt_money(cents);
    let pad = 40usize.saturating_sub(name.len() + amount.len());
    format!("{name}{}{amount}", " ".repeat(pad))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_to_forty() {
        assert_eq!(line("Coffee", 450).len(), 40);
    }
}
