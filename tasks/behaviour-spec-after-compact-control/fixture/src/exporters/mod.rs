use crate::entry::Entry;

pub mod csv;
pub mod json;

/// A text format a ledger can be written in. See SPEC.md, "Exporters".
pub trait Exporter {
    /// The name users pick it by (`--format <name>`).
    fn name(&self) -> &'static str;
    /// The file extension it writes, with the dot.
    fn extension(&self) -> &'static str;
    fn export(&self, entries: &[Entry]) -> String;
}
