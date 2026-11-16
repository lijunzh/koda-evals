#[test]
fn burst() {
    assert_eq!(BURST, 40);
}

#[test]
fn refill() {
    assert_eq!(REFILL_PER_SEC, 12);
    assert_eq!(refill_secs(), 4);
}

#[test]
fn queue() {
    assert_eq!(QUEUE_LIMIT, 2500);
}
