#[test]
fn doubles_from_the_base() {
    let p = Backoff { base_ms: 100, max_ms: 10_000 };
    assert_eq!(delay_for(p, 1), 100);
    assert_eq!(delay_for(p, 2), 200);
    assert_eq!(delay_for(p, 3), 400);
    assert_eq!(delay_for(p, 4), 800);
    assert_eq!(schedule(p, 3), vec![100, 200, 400]);
}

#[test]
fn zero_and_cap_unchanged() {
    let p = Backoff { base_ms: 100, max_ms: 1_000 };
    assert_eq!(delay_for(p, 0), 0);
    assert_eq!(delay_for(p, 5), 1_000);
    assert_eq!(delay_for(p, 100), 1_000);
}
