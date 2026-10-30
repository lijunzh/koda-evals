use std::time::{Duration, Instant};

fn eval_journal() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    std::env::temp_dir().join(format!(
        "relay-eval-{}-{}.log",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ))
}

fn current_thread() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
}

/// Runs `f` on its own thread; `None` when it hasn't finished within
/// `limit` (the thread is left behind, so a hang can't stall the suite).
fn finishes_within<T: Send + 'static>(
    limit: Duration,
    f: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(limit).ok()
}

#[test]
fn concurrent_accepts_of_different_ids_both_land() {
    let path = eval_journal();
    let journal = path.clone();
    let result = finishes_within(Duration::from_secs(10), move || {
        current_thread().block_on(async {
            let (tx, _rx) = tokio::sync::mpsc::channel(8);
            let relay = Relay::open(&journal, Rate::per_second(100), tx)
                .await
                .expect("open");
            let (a, b) = tokio::join!(relay.accept("evt-a"), relay.accept("evt-b"));
            let lines = std::fs::read_to_string(&journal).expect("journal");
            (a.expect("accept a"), b.expect("accept b"), lines)
        })
    });
    let (a, b, lines) = result.expect("two concurrent accepts did not finish within 10 seconds");
    assert_eq!((a, b), (Outcome::Accepted, Outcome::Accepted));
    let mut lines: Vec<&str> = lines.lines().collect();
    lines.sort_unstable();
    assert_eq!(lines, ["evt-a", "evt-b"]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn concurrent_accepts_of_one_id_accept_exactly_once() {
    let path = eval_journal();
    let journal = path.clone();
    let result = finishes_within(Duration::from_secs(10), move || {
        current_thread().block_on(async {
            let (tx, _rx) = tokio::sync::mpsc::channel(8);
            let relay = Relay::open(&journal, Rate::per_second(100), tx)
                .await
                .expect("open");
            let outcomes = tokio::join!(
                relay.accept("evt-x"),
                relay.accept("evt-x"),
                relay.accept("evt-x")
            );
            let outcomes = [
                outcomes.0.expect("accept"),
                outcomes.1.expect("accept"),
                outcomes.2.expect("accept"),
            ];
            let lines = std::fs::read_to_string(&journal).expect("journal");
            (outcomes, relay.health().await, lines)
        })
    });
    let (outcomes, health, lines) =
        result.expect("three concurrent accepts did not finish within 10 seconds");
    let accepted = outcomes.iter().filter(|o| **o == Outcome::Accepted).count();
    assert_eq!(accepted, 1, "outcomes were {outcomes:?}");
    assert_eq!(health, Health { accepted: 1 });
    assert_eq!(lines.lines().collect::<Vec<_>>(), ["evt-x"]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn health_answers_while_deliveries_wait_for_their_slots() {
    // Twelve deliveries at 10/s: the last slot is 1.1 s out. A health
    // check started alongside them must not wait for that.
    let path = eval_journal();
    current_thread().block_on(async {
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        let relay = Relay::open(&path, Rate::per_second(10), tx)
            .await
            .expect("open");
        let started = Instant::now();
        let (delivered_after, health_after) = tokio::join!(
            async {
                for i in 0..12 {
                    relay
                        .deliver(Event::new(format!("evt-{i}"), "{}"))
                        .await
                        .expect("deliver");
                }
                started.elapsed()
            },
            async {
                relay.health().await;
                started.elapsed()
            },
        );
        assert!(
            delivered_after >= Duration::from_millis(1000),
            "rate limit gone: twelve deliveries at 10/s took {delivered_after:?}"
        );
        assert!(
            health_after < Duration::from_millis(400),
            "the health check waited {health_after:?} behind the deliveries"
        );
        let mut ids = Vec::new();
        while let Ok(event) = rx.try_recv() {
            ids.push(event.id);
        }
        let expected: Vec<String> = (0..12).map(|i| format!("evt-{i}")).collect();
        assert_eq!(ids, expected);
    });
    let _ = std::fs::remove_file(&path);
}
