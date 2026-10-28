#[test]
fn non_ascii_first_letters() {
    assert_eq!(initials("élodie dupont"), "ÉD");
    assert_eq!(initials("Ωmega 🐻bear"), "Ω🐻");
}

#[test]
fn ascii_unchanged() {
    assert_eq!(initials("ada lovelace"), "AL");
    assert_eq!(initials(""), "");
}
