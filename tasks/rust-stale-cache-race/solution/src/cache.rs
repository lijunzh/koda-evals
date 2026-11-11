use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A map whose entries expire `ttl` after they were inserted.
///
/// A read that misses takes a [`Ticket`] before it queries the database and
/// fills with [`Cache::fill`]. An invalidation in between bumps the key's
/// generation, so a fill carrying a value read before the write is dropped.
pub struct Cache<K, V> {
    ttl: Duration,
    inner: Mutex<Inner<K, V>>,
}

struct Inner<K, V> {
    entries: HashMap<K, (V, Instant)>,
    generations: HashMap<K, u64>,
}

/// The key's generation when a read missed.
#[derive(Clone, Copy, Debug)]
pub struct Ticket(u64);

impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> {
    pub fn new(ttl: Duration) -> Self {
        Self { ttl, inner: Mutex::new(Inner { entries: HashMap::new(), generations: HashMap::new() }) }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let entries = &mut self.inner.lock().unwrap().entries;
        match entries.get(key) {
            Some((value, at)) if at.elapsed() < self.ttl => Some(value.clone()),
            Some(_) => {
                entries.remove(key);
                None
            }
            None => None,
        }
    }

    pub fn insert(&self, key: K, value: V) {
        self.inner.lock().unwrap().entries.insert(key, (value, Instant::now()));
    }

    /// Take before querying for a missed key.
    pub fn ticket(&self, key: &K) -> Ticket {
        Ticket(self.inner.lock().unwrap().generations.get(key).copied().unwrap_or(0))
    }

    /// Inserts `value` unless `key` was invalidated since `ticket` was taken.
    pub fn fill(&self, key: K, value: V, ticket: Ticket) -> bool {
        let mut inner = self.inner.lock().unwrap();
        if inner.generations.get(&key).copied().unwrap_or(0) != ticket.0 {
            return false;
        }
        inner.entries.insert(key, (value, Instant::now()));
        true
    }

    pub fn invalidate(&self, key: &K) {
        let mut inner = self.inner.lock().unwrap();
        inner.entries.remove(key);
        *inner.generations.entry(key.clone()).or_insert(0) += 1;
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_expire() {
        let cache = Cache::new(Duration::from_millis(20));
        cache.insert(1, "a");
        assert_eq!(cache.get(&1), Some("a"));
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(cache.get(&1), None);
        assert!(cache.is_empty());
    }

    #[test]
    fn a_fill_from_before_an_invalidation_is_dropped() {
        let cache = Cache::new(Duration::from_secs(60));
        let ticket = cache.ticket(&1);
        cache.invalidate(&1);
        assert!(!cache.fill(1, "old", ticket));
        assert_eq!(cache.get(&1), None);
        let ticket = cache.ticket(&1);
        assert!(cache.fill(1, "new", ticket));
        assert_eq!(cache.get(&1), Some("new"));
    }

    #[test]
    fn invalidate_removes() {
        let cache = Cache::new(Duration::from_secs(60));
        cache.insert(1, "a");
        cache.invalidate(&1);
        assert_eq!(cache.get(&1), None);
    }
}
