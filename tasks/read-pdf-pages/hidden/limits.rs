#[test]
fn retries() {
    assert_eq!(MAX_RETRIES, 6);
}

#[test]
fn backoff_starts_at_the_agreed_delay() {
    assert_eq!(BACKOFF_MS, 375);
    assert_eq!(backoff(2), 1500);
}

#[test]
fn body_cap() {
    assert_eq!(MAX_BODY_BYTES, 640 * 1024);
}
