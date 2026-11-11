use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

pub type UserId = u64;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub id: UserId,
    pub name: String,
    pub email: String,
}

#[derive(Debug)]
pub struct StoreError(pub String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "store: {}", self.0)
    }
}

/// The profile database. Production uses the Postgres implementation in
/// another crate, so this trait can't change.
///
/// Each call is one query. A load returns the rows as they were when the
/// query ran; the result can arrive later, after other queries have changed
/// them.
pub trait Store: Send + Sync {
    fn load(&self, id: UserId) -> BoxFuture<'_, Result<Option<Profile>, StoreError>>;
    /// The profiles that exist among `ids`, in any order.
    fn load_many<'a>(&'a self, ids: &'a [UserId]) -> BoxFuture<'a, Result<Vec<Profile>, StoreError>>;
    fn save<'a>(&'a self, profile: &'a Profile) -> BoxFuture<'a, Result<(), StoreError>>;
    fn delete(&self, id: UserId) -> BoxFuture<'_, Result<(), StoreError>>;
}

/// An in-memory store for tests and local runs.
#[derive(Default)]
pub struct MemStore {
    rows: Mutex<HashMap<UserId, Profile>>,
    loads: Mutex<usize>,
}

impl MemStore {
    pub fn with(profiles: impl IntoIterator<Item = Profile>) -> Self {
        let store = Self::default();
        store.rows.lock().unwrap().extend(profiles.into_iter().map(|p| (p.id, p)));
        store
    }

    /// Queries that read rows: `load` and `load_many` calls.
    pub fn loads(&self) -> usize {
        *self.loads.lock().unwrap()
    }
}

impl Store for MemStore {
    fn load(&self, id: UserId) -> BoxFuture<'_, Result<Option<Profile>, StoreError>> {
        *self.loads.lock().unwrap() += 1;
        let row = self.rows.lock().unwrap().get(&id).cloned();
        Box::pin(async move { Ok(row) })
    }

    fn load_many<'a>(&'a self, ids: &'a [UserId]) -> BoxFuture<'a, Result<Vec<Profile>, StoreError>> {
        *self.loads.lock().unwrap() += 1;
        let rows = self.rows.lock().unwrap();
        let found = ids.iter().filter_map(|id| rows.get(id).cloned()).collect();
        Box::pin(async move { Ok(found) })
    }

    fn save<'a>(&'a self, profile: &'a Profile) -> BoxFuture<'a, Result<(), StoreError>> {
        self.rows.lock().unwrap().insert(profile.id, profile.clone());
        Box::pin(async { Ok(()) })
    }

    fn delete(&self, id: UserId) -> BoxFuture<'_, Result<(), StoreError>> {
        self.rows.lock().unwrap().remove(&id);
        Box::pin(async { Ok(()) })
    }
}
