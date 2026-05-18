use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use factorio_api::PortalLicenseId;
use tokio::sync::{Mutex, RwLock};

use super::DedupString;

#[derive(Clone)]
pub struct AttributionDataInternal {
    pub title: String,
    pub owner: DedupString,
    pub license: LicenseInfo,
    pub source_url: Option<String>,
}

#[derive(Clone)]
pub struct LicenseInfo {
    pub title: DedupString,
    pub url: DedupString,
    pub access_allowed: bool,
}

#[derive(Default)]
pub struct AttributionCache {
    cache: RwLock<HashMap<String, AttributionDataInternal>>,
    fetch_permits: RwLock<HashMap<String, Arc<Mutex<()>>>>,

    all_owners: RwLock<HashSet<DedupString>>,
    all_license_titles: RwLock<HashSet<DedupString>>,
    all_license_urls: RwLock<HashSet<DedupString>>,
}

impl AttributionCache {
    pub async fn get(&self, name: &str) -> Option<AttributionDataInternal> {
        // Acquire the permit for this mod to prevent multiple concurrent fetches
        let permit = self.get_permit(name).await;
        let guard = permit.lock().await;

        if let Some(data) = self.cache.read().await.get(name) {
            return Some(data.clone());
        }

        let res = self.fetch_and_cache(name).await;
        drop(guard);
        res
    }

    pub async fn can_access(&self, name: &str) -> bool {
        self.get(name)
            .await
            .is_some_and(|data| data.license.access_allowed)
    }

    async fn get_permit(&self, name: &str) -> Arc<Mutex<()>> {
        let mut permits = self.fetch_permits.write().await;
        if let Some(permit) = permits.get(name).cloned() {
            return permit;
        }

        let permit = Arc::new(Mutex::new(()));
        permits.insert(name.to_string(), permit.clone());

        permit
    }

    async fn fetch_and_cache(&self, name: &str) -> Option<AttributionDataInternal> {
        let data = match factorio_api::full_info(name).await {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to fetch mod info for {name}: {e}");
                return None;
            }
        };

        let owner = dedup_helper(&data.owner, &self.all_owners).await;
        let title = dedup_helper(&data.license.title, &self.all_license_titles).await;
        let url = dedup_helper(&data.license.url, &self.all_license_urls).await;

        let license = LicenseInfo {
            title,
            url,
            access_allowed: data.license.id != PortalLicenseId::Other, // TODO: inspect the license name / title to determine if it's actually allowed or not
        };

        let attribution = AttributionDataInternal {
            title: data.title,
            owner,
            license,
            source_url: data.source_url,
        };

        self.cache
            .write()
            .await
            .insert(name.to_string(), attribution.clone());

        Some(attribution)
    }
}

async fn dedup_helper(val: &str, mem: &RwLock<HashSet<DedupString>>) -> DedupString {
    if let Some(existing) = mem.read().await.get(val).cloned() {
        return existing;
    }

    let val: DedupString = val.into();
    mem.write().await.insert(val.clone());
    val
}
