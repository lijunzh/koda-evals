use std::sync::Arc;
use std::time::Duration;

use crate::cache::Cache;
use crate::store::{Profile, Store, StoreError, UserId};

#[derive(Debug)]
pub enum Error {
    Store(StoreError),
    NotFound(UserId),
    InvalidEmail(String),
}

impl From<StoreError> for Error {
    fn from(e: StoreError) -> Self {
        Error::Store(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Store(e) => write!(f, "{e}"),
            Error::NotFound(id) => write!(f, "no user {id}"),
            Error::InvalidEmail(e) => write!(f, "not an email address: {e:?}"),
        }
    }
}

/// Profiles, read through a cache. Reads are served from the cache when
/// they can be, so most page views never reach the database. Writes go to
/// the database first and then drop the cached copy, so the next read
/// loads the new row.
pub struct Profiles {
    store: Arc<dyn Store>,
    cache: Cache<UserId, Profile>,
}

impl Profiles {
    pub fn new(store: Arc<dyn Store>, ttl: Duration) -> Self {
        Self { store, cache: Cache::new(ttl) }
    }

    /// One profile; `None` when the user doesn't exist.
    pub async fn profile(&self, id: UserId) -> Result<Option<Profile>, Error> {
        if let Some(p) = self.cache.get(&id) {
            return Ok(Some(p));
        }
        // The ticket comes before the query: a write that lands while the
        // query is in flight invalidates it, and the fill is dropped.
        let ticket = self.cache.ticket(&id);
        let loaded = self.store.load(id).await?;
        if let Some(p) = &loaded {
            self.cache.fill(id, p.clone(), ticket);
        }
        Ok(loaded)
    }

    /// The profiles that exist among `ids`, in the order asked for. Misses
    /// are loaded with one query.
    pub async fn profiles(&self, ids: &[UserId]) -> Result<Vec<Profile>, Error> {
        let mut found: Vec<Option<Profile>> = ids.iter().map(|id| self.cache.get(id)).collect();
        let missing: Vec<UserId> = ids
            .iter()
            .zip(&found)
            .filter(|(_, hit)| hit.is_none())
            .map(|(id, _)| *id)
            .collect();
        if !missing.is_empty() {
            let tickets: Vec<_> = missing.iter().map(|id| (*id, self.cache.ticket(id))).collect();
            for p in self.store.load_many(&missing).await? {
                if let Some((_, ticket)) = tickets.iter().find(|(id, _)| *id == p.id) {
                    self.cache.fill(p.id, p.clone(), *ticket);
                }
                for (slot, id) in found.iter_mut().zip(ids) {
                    if *id == p.id && slot.is_none() {
                        *slot = Some(p.clone());
                    }
                }
            }
        }
        Ok(found.into_iter().flatten().collect())
    }

    pub async fn update_email(&self, id: UserId, email: &str) -> Result<Profile, Error> {
        let email = email.trim();
        if !email.contains('@') || email.contains(char::is_whitespace) {
            return Err(Error::InvalidEmail(email.to_string()));
        }
        let mut profile = self.store.load(id).await?.ok_or(Error::NotFound(id))?;
        profile.email = email.to_string();
        self.store.save(&profile).await?;
        self.cache.invalidate(&id);
        Ok(profile)
    }

    pub async fn delete(&self, id: UserId) -> Result<(), Error> {
        self.store.delete(id).await?;
        self.cache.invalidate(&id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemStore;

    fn alice() -> Profile {
        Profile { id: 1, name: "Alice".into(), email: "alice@old.example".into() }
    }

    fn bob() -> Profile {
        Profile { id: 2, name: "Bob".into(), email: "bob@example.com".into() }
    }

    fn setup() -> (Arc<MemStore>, Profiles) {
        let store = Arc::new(MemStore::with([alice(), bob()]));
        let profiles = Profiles::new(store.clone(), Duration::from_secs(3600));
        (store, profiles)
    }

    #[tokio::test]
    async fn a_second_read_comes_from_the_cache() {
        let (store, profiles) = setup();
        assert_eq!(profiles.profile(1).await.unwrap(), Some(alice()));
        assert_eq!(profiles.profile(1).await.unwrap(), Some(alice()));
        assert_eq!(store.loads(), 1);
    }

    #[tokio::test]
    async fn a_read_after_an_update_sees_the_new_email() {
        let (_, profiles) = setup();
        profiles.profile(1).await.unwrap();
        profiles.update_email(1, "alice@new.example").await.unwrap();
        let p = profiles.profile(1).await.unwrap().unwrap();
        assert_eq!(p.email, "alice@new.example");
    }

    #[tokio::test]
    async fn a_deleted_user_is_gone() {
        let (_, profiles) = setup();
        profiles.profile(1).await.unwrap();
        profiles.delete(1).await.unwrap();
        assert_eq!(profiles.profile(1).await.unwrap(), None);
    }

    #[tokio::test]
    async fn a_batch_keeps_the_order_and_skips_unknown_users() {
        let (store, profiles) = setup();
        profiles.profile(2).await.unwrap();
        let got = profiles.profiles(&[2, 9, 1]).await.unwrap();
        assert_eq!(got, vec![bob(), alice()]);
        assert_eq!(store.loads(), 2);
        profiles.profiles(&[1, 2]).await.unwrap();
        assert_eq!(store.loads(), 2, "both now come from the cache");
    }

    #[tokio::test]
    async fn a_bad_email_is_refused() {
        let (_, profiles) = setup();
        assert!(matches!(profiles.update_email(1, "nope").await, Err(Error::InvalidEmail(_))));
        assert!(matches!(profiles.update_email(7, "x@y.z").await, Err(Error::NotFound(7))));
    }
}
