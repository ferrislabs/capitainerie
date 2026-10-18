use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

use capitainerie_common::OrganizationId;
use capitainerie_core::{CapitainerieUseCase, application::ModelListing};

pub const CATALOG_TTL: Duration = Duration::from_secs(10 * 60);

pub struct ProvidersState<C> {
    pub usecase: CapitainerieUseCase,
    pub catalog: Arc<C>,
    pub cache: Arc<CatalogCache>,
}

impl<C> ProvidersState<C> {
    pub fn new(usecase: CapitainerieUseCase, catalog: C) -> Self {
        Self {
            usecase,
            catalog: Arc::new(catalog),
            cache: Arc::new(CatalogCache::new(CATALOG_TTL)),
        }
    }
}

impl<C> Clone for ProvidersState<C> {
    fn clone(&self) -> Self {
        Self {
            usecase: self.usecase.clone(),
            catalog: Arc::clone(&self.catalog),
            cache: Arc::clone(&self.cache),
        }
    }
}

/// Providers' catalogs per organization, kept for `ttl` so the console does
/// not call every provider on each page view. Process-local.
pub struct CatalogCache {
    ttl: Duration,
    entries: Mutex<HashMap<OrganizationId, (Instant, ModelListing)>>,
}

impl CatalogCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, organization_id: OrganizationId, now: Instant) -> Option<ModelListing> {
        let entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        entries
            .get(&organization_id)
            .filter(|(stored_at, _)| now.saturating_duration_since(*stored_at) < self.ttl)
            .map(|(_, listing)| listing.clone())
    }

    pub fn put(&self, organization_id: OrganizationId, listing: ModelListing, now: Instant) {
        self.entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(organization_id, (now, listing));
    }

    pub fn invalidate(&self, organization_id: OrganizationId) {
        self.entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&organization_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_lives_for_the_ttl_and_only_for_its_organization() {
        let cache = CatalogCache::new(Duration::from_secs(600));
        let (acme, other) = (OrganizationId::new(), OrganizationId::new());
        let start = Instant::now();
        cache.put(acme, ModelListing::default(), start);

        assert!(cache.get(acme, start + Duration::from_secs(599)).is_some());
        assert!(cache.get(acme, start + Duration::from_secs(600)).is_none());
        assert!(cache.get(other, start).is_none());
    }

    #[test]
    fn invalidating_drops_only_that_organization() {
        let cache = CatalogCache::new(Duration::from_secs(600));
        let (acme, other) = (OrganizationId::new(), OrganizationId::new());
        let now = Instant::now();
        cache.put(acme, ModelListing::default(), now);
        cache.put(other, ModelListing::default(), now);

        cache.invalidate(acme);

        assert!(cache.get(acme, now).is_none());
        assert!(cache.get(other, now).is_some());
    }
}
