use crate::entry::Entry;
use crate::registry::{by_name, exporters};

fn run(name: &str, entries: &[Entry]) -> String {
    by_name(name).unwrap_or_else(|| panic!("no exporter named {name:?}")).export(entries)
}

#[test]
fn the_registry_is_sorted_by_name() {
    let names: Vec<&str> = exporters().iter().map(|e| e.name()).collect();
    assert_eq!(names, ["csv", "fixed", "jsonl", "markdown", "tsv"]);
}

#[test]
fn extensions() {
    let ext = |n: &str| by_name(n).map(|e| e.extension());
    assert_eq!(ext("tsv"), Some(".tab"));
    assert_eq!(ext("markdown"), Some(".md"));
    assert_eq!(ext("fixed"), Some(".txt"));
    assert_eq!(ext("csv"), Some(".csv"));
}

#[test]
fn tsv_header_escapes_and_cents() {
    let out = run("tsv", &[
        Entry::new("2026-03-01", "Rent\tMarch", -150000),
        Entry::new("2026-03-02", "a\\b\nc\rd", 5),
    ]);
    assert_eq!(out, "DATE\tPAYEE\tAMOUNT\n2026-03-01\tRent\\tMarch\t-150000\n2026-03-02\ta\\\\b\\nc\\rd\t5\n");
    assert_eq!(run("tsv", &[]), "DATE\tPAYEE\tAMOUNT\n");
}

#[test]
fn markdown_alignment_escapes_and_total() {
    let out = run("markdown", &[
        Entry::new("2026-03-01", "A|B", -1250),
        Entry::new("2026-03-02", "two\nlines", 100),
    ]);
    assert_eq!(
        out,
        "| Date | Payee | Amount |\n| --- | --- | ---: |\n| 2026-03-01 | A\\|B | -12.50 |\n| 2026-03-02 | two<br>lines | 1.00 |\n| | **Total** | **-11.50** |\n"
    );
    assert_eq!(run("markdown", &[]), "| Date | Payee | Amount |\n| --- | --- | ---: |\n| | **Total** | **0.00** |\n");
}

#[test]
fn fixed_width_truncation_padding_and_crlf() {
    let out = run("fixed", &[
        Entry::new("2026-03-01", "Café Zürich", -450),
        Entry::new("2026-03-02", "Municipal Water Authority of Springfield", 123456),
        Entry::new("2026-03-03", "tab\there", 0),
    ]);
    let lines: Vec<&str> = out.split_inclusive("\r\n").collect();
    assert_eq!(lines.len(), 3, "{out:?}");
    assert_eq!(lines[0], "2026-03-01  Café Zürich                      -4.50\r\n");
    assert_eq!(lines[1], "2026-03-02  Municipal Water Authori…       1234.56\r\n");
    assert_eq!(lines[2], "2026-03-03  tab here                          0.00\r\n");
    for l in &lines {
        assert_eq!(l.trim_end_matches("\r\n").chars().count(), 10 + 2 + 24 + 2 + 12, "{l:?}");
    }
    assert_eq!(run("fixed", &[]), "");
}

#[test]
fn existing_formats_unchanged() {
    assert_eq!(run("csv", &[Entry::new("2026-01-02", "Smith, J", -1250)]), "date,payee,amount\r\n2026-01-02,\"Smith, J\",-12.50\r\n");
    assert_eq!(run("jsonl", &[]), "");
}
