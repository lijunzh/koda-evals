#[test]
fn only_saturday_and_sunday_are_weekend() {
    for day in Weekday::ALL {
        let expected = matches!(day, Weekday::Saturday | Weekday::Sunday);
        assert_eq!(is_weekend(day), expected, "{day:?}");
    }
}

#[test]
fn a_full_week_counts_two() {
    assert_eq!(weekend_days(Weekday::Monday, 7), 2);
    assert_eq!(weekend_days(Weekday::Friday, 3), 2);
    assert_eq!(weekend_days(Weekday::Monday, 5), 0);
}
