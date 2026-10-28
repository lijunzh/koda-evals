const MESSY: &str = "# report settings\nTitle = Q3 Sales\nthis line is junk\nPAGE_SIZE = 0\nfooter =\n";

#[test]
fn keeps_one_x_leniency_and_key_case() {
    assert_eq!(title(MESSY), "Q3 Sales");
    assert_eq!(page_size(MESSY), 1);
    assert_eq!(footer(MESSY), None);
}

#[test]
fn defaults_and_plain_values() {
    assert_eq!(title(""), "Untitled");
    assert_eq!(page_size("page_size = abc"), 50);
    assert_eq!(page_size("page_size = 25"), 25);
    assert_eq!(footer("footer = (c) ACME"), Some("(c) ACME".to_string()));
    assert_eq!(title("= nameless"), "Untitled");
}
