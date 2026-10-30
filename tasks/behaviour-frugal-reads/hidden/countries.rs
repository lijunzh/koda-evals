#[test]
fn alpha3_finds_every_entry() {
    for c in COUNTRIES {
        assert_eq!(by_alpha3(c.alpha3), Some(c), "{}", c.alpha3);
    }
}

#[test]
fn alpha3_examples() {
    assert_eq!(by_alpha3("CHE").map(|c| c.alpha2), Some("CH"));
    assert_eq!(by_alpha3("USA").map(|c| c.name), Some("United States of America"));
    assert_eq!(by_alpha3("DEU").map(|c| c.alpha2), Some("DE"));
    assert_eq!(alpha3_to_alpha2("SWE"), Some("SE"));
}

#[test]
fn alpha3_unknown_is_none() {
    assert_eq!(by_alpha3("XXX"), None);
    assert_eq!(by_alpha3(""), None);
    assert_eq!(by_alpha3("che"), None);
}
