#[test]
fn compound_durations() {
    assert_eq!(parse_duration("1h30m"), Ok(Duration::from_secs(5400)));
    assert_eq!(parse_duration("1h30m15s"), Ok(Duration::from_secs(5415)));
    assert_eq!(parse_duration("2m30s"), Ok(Duration::from_secs(150)));
}

#[test]
fn single_units_and_errors_unchanged() {
    assert_eq!(parse_duration("90m"), Ok(Duration::from_secs(5400)));
    assert_eq!(parse_duration("45s"), Ok(Duration::from_secs(45)));
    assert_eq!(parse_duration("2h"), Ok(Duration::from_secs(7200)));
    assert!(parse_duration("").is_err());
    assert!(parse_duration("10").is_err());
    assert!(parse_duration("ten minutes").is_err());
}
