#[test]
fn non_ascii_local_part() {
    assert_eq!(mask_email("élodie@example.fr"), "é***@example.fr");
    assert_eq!(mask_email("🐻@zoo.org"), "🐻***@zoo.org");
}

#[test]
fn edge_cases_unchanged() {
    assert_eq!(mask_email("jane@example.com"), "j***@example.com");
    assert_eq!(mask_email("not-an-email"), "not-an-email");
    assert_eq!(mask_email("@example.com"), "@example.com");
}
