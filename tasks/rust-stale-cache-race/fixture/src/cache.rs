use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A map whose entries expire `ttl` after they were inserted.
pub struct Cache<K, V> {
    ttl: Duration,
    entries: Mutex<HashMap<K, (V, Instant)>>,
}

impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> {
    pub fn new(ttl: Duration) -> Self {
        Self { ttl, entries: Mutex::new(HashMap::new()) }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.lock().unwrap();
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
        self.entries.lock().unwrap().insert(key, (value, Instant::now()));
    }

    pub fn invalidate(&self, key: &K) {
        self.entries.lock().unwrap().remove(key);
    }

    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
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
    fn invalidate_removes() {
        let cache = Cache::new(Duration::from_secs(60));
        cache.insert(1, "a");
        cache.invalidate(&1);
        assert_eq!(cache.get(&1), None);
    }
}
