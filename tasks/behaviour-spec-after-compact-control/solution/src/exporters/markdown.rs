use super::Exporter;
use crate::entry::{Entry, decimal};

/// A GitHub-flavoured table with a total row. See SPEC.md, "Markdown table".
pub struct Markdown;

impl Exporter for Markdown {
    fn name(&self) -> &'static str {
        "markdown"
    }
    fn extension(&self) -> &'static str {
        ".md"
    }
    fn export(&self, entries: &[Entry]) -> String {
        let mut out = String::from("| Date | Payee | Amount |\n| --- | --- | ---: |\n");
        for e in entries {
            let payee = e.payee.replace('|', "\\|").replace('\n', "<br>");
            out.push_str(&format!("| {} | {} | {} |\n", e.date, payee, decimal(e.amount_cents)));
        }
        let total: i64 = entries.iter().map(|e| e.amount_cents).sum();
        out.push_str(&format!("| | **Total** | **{}** |\n", decimal(total)));
        out
    }
}
