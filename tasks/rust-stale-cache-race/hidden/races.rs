// Appended to src/lib.rs inside `mod eval_hidden { use super::*; ... }`.
//
// A store whose loads can be held open: a load reads the rows when it
// starts (as a query does) and returns only when the test releases it. That
// forces the interleaving production hits by chance: a read's query runs,
// a write lands, then the read's result comes back.
//
// The tests accept any correct design. A write may wait for a read in
// flight on the same user (a per-user lock), or the read may finish and
// leave the cache alone (a version check): the test gives the write up to
// half a second to land before it releases the read, and only then checks
// what a fresh read returns. Every test runs under a ten-second limit, so a
// deadlock fails instead of hanging.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use tokio::sync::Semaphore;

struct Gated {
    rows: StdMutex<HashMap<UserId, Profile>>,
    /// Loads that start while this is set wait for `gate`.
    hold: AtomicBool,
    gate: Semaphore,
    held: AtomicUsize,
    loads: AtomicUsize,
    writes: AtomicUsize,
}

impl Gated {
    fn with(profiles: impl IntoIterator<Item = Profile>) -> Arc<Self> {
        let g = Gated {
            rows: StdMutex::new(HashMap::new()),
            hold: AtomicBool::new(false),
            gate: Semaphore::new(0),
            held: AtomicUsize::new(0),
            loads: AtomicUsize::new(0),
            writes: AtomicUsize::new(0),
        };
        g.rows.lock().unwrap().extend(profiles.into_iter().map(|p| (p.id, p)));
        Arc::new(g)
    }

    async fn maybe_wait(&self) {
        if self.hold.load(Ordering::SeqCst) {
            self.held.fetch_add(1, Ordering::SeqCst);
            self.gate.acquire().await.expect("gate").forget();
        }
    }
}

impl Store for Gated {
    fn load(&self, id: UserId) -> BoxFuture<'_, Result<Option<Profile>, StoreError>> {
        self.loads.fetch_add(1, Ordering::SeqCst);
        let row = self.rows.lock().unwrap().get(&id).cloned();
        Box::pin(async move {
            self.maybe_wait().await;
            Ok(row)
        })
    }

    fn load_many<'a>(&'a self, ids: &'a [UserId]) -> BoxFuture<'a, Result<Vec<Profile>, StoreError>> {
        self.loads.fetch_add(1, Ordering::SeqCst);
        let rows: Vec<Profile> = {
            let rows = self.rows.lock().unwrap();
            ids.iter().filter_map(|id| rows.get(id).cloned()).collect()
        };
        Box::pin(async move {
            self.maybe_wait().await;
            Ok(rows)
        })
    }

    fn save<'a>(&'a self, profile: &'a Profile) -> BoxFuture<'a, Result<(), StoreError>> {
        let profile = profile.clone();
        Box::pin(async move {
            self.rows.lock().unwrap().insert(profile.id, profile);
            self.writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }

    fn delete(&self, id: UserId) -> BoxFuture<'_, Result<(), StoreError>> {
        Box::pin(async move {
            self.rows.lock().unwrap().remove(&id);
            self.writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
}

fn alice() -> Profile {
    Profile { id: 1, name: "Alice".into(), email: "alice@old.example".into() }
}

fn bob() -> Profile {
    Profile { id: 2, name: "Bob".into(), email: "bob@example.com".into() }
}

/// Polls until `cond` holds or `limit` passes; returns whether it held.
async fn eventually(limit: Duration, cond: impl Fn() -> bool) -> bool {
    let deadline = tokio::time::Instant::now() + limit;
    while tokio::time::Instant::now() < deadline {
        if cond() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    cond()
}

async fn within_ten_seconds<T>(f: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(10), f)
        .await
        .expect("did not finish within 10 seconds (a deadlock?)")
}

/// Starts `read` with its load held, runs `write` while it is held (or
/// waits up to half a second for the write to land), then lets the read
/// finish.
async fn read_across_write<R, W>(store: &Arc<Gated>, read: R, write: W)
where
    R: std::future::Future + Send + 'static,
    R::Output: Send,
    W: std::future::Future + Send + 'static,
    W::Output: Send,
{
    store.hold.store(true, Ordering::SeqCst);
    let reader = tokio::spawn(read);
    assert!(
        eventually(Duration::from_secs(5), || store.held.load(Ordering::SeqCst) == 1).await,
        "the read never reached the store"
    );
    store.hold.store(false, Ordering::SeqCst);
    let writes_before = store.writes.load(Ordering::SeqCst);
    let writer = tokio::spawn(write);
    eventually(Duration::from_millis(500), || store.writes.load(Ordering::SeqCst) > writes_before).await;
    store.gate.add_permits(1);
    reader.await.expect("read task");
    writer.await.expect("write task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_read_in_flight_during_an_email_change_does_not_cache_the_old_email() {
    within_ten_seconds(async {
        let store = Gated::with([alice(), bob()]);
        let profiles = Arc::new(Profiles::new(store.clone(), Duration::from_secs(3600)));
        let (p1, p2) = (profiles.clone(), profiles.clone());
        read_across_write(
            &store,
            async move { p1.profile(1).await.expect("read") },
            async move { p2.update_email(1, "alice@new.example").await.expect("update") },
        )
        .await;
        let now = profiles.profile(1).await.expect("read").expect("alice");
        assert_eq!(now.email, "alice@new.example", "a read after the change returned the old email");
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_batch_read_in_flight_during_an_email_change_does_not_cache_the_old_email() {
    within_ten_seconds(async {
        let store = Gated::with([alice(), bob()]);
        let profiles = Arc::new(Profiles::new(store.clone(), Duration::from_secs(3600)));
        let (p1, p2) = (profiles.clone(), profiles.clone());
        read_across_write(
            &store,
            async move { p1.profiles(&[2, 1]).await.expect("batch read") },
            async move { p2.update_email(1, "alice@new.example").await.expect("update") },
        )
        .await;
        let now = profiles.profile(1).await.expect("read").expect("alice");
        assert_eq!(now.email, "alice@new.example", "a read after the change returned the old email");
        let batch = profiles.profiles(&[1, 2]).await.expect("batch read");
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].email, "alice@new.example");
        assert_eq!(batch[1], bob());
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_read_in_flight_during_a_delete_does_not_bring_the_user_back() {
    within_ten_seconds(async {
        let store = Gated::with([alice(), bob()]);
        let profiles = Arc::new(Profiles::new(store.clone(), Duration::from_secs(3600)));
        let (p1, p2) = (profiles.clone(), profiles.clone());
        read_across_write(
            &store,
            async move { p1.profile(1).await.expect("read") },
            async move { p2.delete(1).await.expect("delete") },
        )
        .await;
        assert_eq!(profiles.profile(1).await.expect("read"), None, "the deleted user came back");
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reads_still_come_from_the_cache() {
    within_ten_seconds(async {
        let store = Gated::with([alice(), bob()]);
        let profiles = Profiles::new(store.clone(), Duration::from_secs(3600));
        profiles.profile(1).await.expect("read");
        profiles.profile(1).await.expect("read");
        profiles.profiles(&[1, 2]).await.expect("batch read");
        profiles.profiles(&[2, 1]).await.expect("batch read");
        profiles.profile(2).await.expect("read");
        assert_eq!(store.loads.load(Ordering::SeqCst), 2, "repeat reads went to the database");
        profiles.update_email(1, "alice@new.example").await.expect("update");
        let loads = store.loads.load(Ordering::SeqCst);
        profiles.profile(1).await.expect("read");
        profiles.profile(1).await.expect("read");
        assert!(
            store.loads.load(Ordering::SeqCst) <= loads + 1,
            "the reads after an update weren't cached again"
        );
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reads_of_different_users_do_not_wait_on_each_other() {
    within_ten_seconds(async {
        let store = Gated::with([alice(), bob()]);
        let profiles = Arc::new(Profiles::new(store.clone(), Duration::from_secs(3600)));
        store.hold.store(true, Ordering::SeqCst);
        let (p1, p2) = (profiles.clone(), profiles.clone());
        let a = tokio::spawn(async move { p1.profile(1).await.expect("read") });
        let b = tokio::spawn(async move { p2.profile(2).await.expect("read") });
        let both = eventually(Duration::from_secs(2), || store.held.load(Ordering::SeqCst) == 2).await;
        store.hold.store(false, Ordering::SeqCst);
        store.gate.add_permits(2);
        a.await.expect("read task");
        b.await.expect("read task");
        assert!(both, "the second user's read waited for the first user's query");
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_update_while_nothing_is_in_flight_still_works() {
    within_ten_seconds(async {
        let store = Gated::with([alice(), bob()]);
        let profiles = Profiles::new(store.clone(), Duration::from_secs(3600));
        profiles.profile(1).await.expect("read");
        let updated = profiles.update_email(1, " alice@new.example ").await.expect("update");
        assert_eq!(updated.email, "alice@new.example");
        assert_eq!(profiles.profile(1).await.expect("read").expect("alice").email, "alice@new.example");
        profiles.delete(2).await.expect("delete");
        assert_eq!(profiles.profiles(&[1, 2]).await.expect("batch read").len(), 1);
    })
    .await;
}
