/// A table header: the column names joined by ` | ` with the first one
/// upper-cased as the row label. An empty column list gives an empty
/// string.
pub fn header(cols: &[&str]) -> String {
    let mut out = cols[0].to_uppercase();
    for col in &cols[1..] {
        out.push_str(" | ");
        out.push_str(col);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_columns() {
        assert_eq!(header(&["name", "qty", "price"]), "NAME | qty | price");
    }
}
