#[test]
fn retries() {
    assert_eq!(MAX_RETRIES, 5);
}

#[test]
fn backoff_starts_at_the_agreed_delay() {
    assert_eq!(BACKOFF_MS, 425);
    assert_eq!(backoff(2), 1700);
}

#[test]
fn body_cap() {
    assert_eq!(MAX_BODY_BYTES, 768 * 1024);
}
