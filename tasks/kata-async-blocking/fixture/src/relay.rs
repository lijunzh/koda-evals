//! The request handlers: accept, deliver, health.

use std::collections::HashSet;
use std::io;
use std::path::Path;
use std::sync::Mutex;

use tokio::sync::mpsc;

use crate::journal::Journal;
use crate::limiter::{Limiter, Rate};

/// An event bound for the downstream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: String,
    pub payload: String,
}

impl Event {
    pub fn new(id: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            payload: payload.into(),
        }
    }
}

/// What [`Relay::accept`] decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// First time this id was seen; it is journaled.
    Accepted,
    /// Seen before (possibly before a restart); nothing recorded.
    Duplicate,
}

/// What the health endpoint reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    /// Ids accepted so far, including those reloaded from the journal.
    pub accepted: usize,
}

pub struct Relay {
    seen: Mutex<HashSet<String>>,
    journal: Journal,
    limiter: Limiter,
    outbound: mpsc::Sender<Event>,
}

impl Relay {
    /// Opens the journal at `journal` (reloading accepted ids from it) and
    /// delivers events to `outbound`, the sender pool's queue, at `rate`.
    pub async fn open(
        journal: &Path,
        rate: Rate,
        outbound: mpsc::Sender<Event>,
    ) -> io::Result<Self> {
        let (reloaded, journal) = Journal::open(journal).await?;
        Ok(Self {
            seen: Mutex::new(reloaded.into_iter().collect()),
            journal,
            limiter: Limiter::new(rate),
            outbound,
        })
    }

    /// Records `id` once. The journal line is written before the id counts
    /// as seen, so an acknowledged accept survives a restart. Of concurrent
    /// accepts with the same id exactly one is `Accepted`.
    pub async fn accept(&self, id: &str) -> io::Result<Outcome> {
        let mut seen = self.seen.lock().expect("seen lock");
        if seen.contains(id) {
            return Ok(Outcome::Duplicate);
        }
        self.journal.append(id).await?;
        seen.insert(id.to_owned());
        Ok(Outcome::Accepted)
    }

    /// Hands `event` to the sender pool, no faster than the configured
    /// rate. Returns the event when the pool has shut down.
    pub async fn deliver(&self, event: Event) -> Result<(), Event> {
        self.limiter.acquire().await;
        self.outbound.send(event).await.map_err(|e| e.0)
    }

    /// Cheap status for the health endpoint.
    pub async fn health(&self) -> Health {
        Health {
            accepted: self.seen.lock().expect("seen lock").len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_journal() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        std::env::temp_dir().join(format!(
            "relay-test-{}-{}.log",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[tokio::test]
    async fn accepts_an_id_once() {
        let path = scratch_journal();
        let (tx, _rx) = mpsc::channel(8);
        let relay = Relay::open(&path, Rate::per_second(10), tx).await.unwrap();
        assert_eq!(relay.accept("evt-1").await.unwrap(), Outcome::Accepted);
        assert_eq!(relay.accept("evt-1").await.unwrap(), Outcome::Duplicate);
        assert_eq!(relay.health().await, Health { accepted: 1 });
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn accepted_ids_survive_a_reopen() {
        let path = scratch_journal();
        let (tx, _rx) = mpsc::channel(8);
        {
            let relay = Relay::open(&path, Rate::per_second(10), tx.clone()).await.unwrap();
            relay.accept("evt-1").await.unwrap();
            relay.accept("evt-2").await.unwrap();
        }
        let relay = Relay::open(&path, Rate::per_second(10), tx).await.unwrap();
        assert_eq!(relay.accept("evt-2").await.unwrap(), Outcome::Duplicate);
        assert_eq!(relay.health().await, Health { accepted: 2 });
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn delivers_in_order() {
        let path = scratch_journal();
        let (tx, mut rx) = mpsc::channel(8);
        let relay = Relay::open(&path, Rate::per_second(1000), tx).await.unwrap();
        relay.deliver(Event::new("a", "{}")).await.unwrap();
        relay.deliver(Event::new("b", "{}")).await.unwrap();
        assert_eq!(rx.recv().await.unwrap().id, "a");
        assert_eq!(rx.recv().await.unwrap().id, "b");
        let _ = std::fs::remove_file(&path);
    }
}
