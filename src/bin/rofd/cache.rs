//! Owned raster sources, budgeted by combined decoded and encoded bytes.
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderKey {
    pub page: usize,
    pub scale: u32,
    pub thumbnail: bool,
}
impl RenderKey {
    pub fn new(page: usize, scale: f64, thumbnail: bool) -> Self {
        Self {
            page,
            scale: (scale.clamp(0.01, 16.0) * 100.0).round() as u32,
            thumbnail,
        }
    }
    pub fn scale(self) -> f64 {
        self.scale as f64 / 100.0
    }
}

struct Entry {
    key: RenderKey,
    source: Arc<String>,
    bytes: u64,
    reduced: bool,
}
pub struct RasterCache {
    entries: std::collections::VecDeque<Entry>,
    budget: u64,
    bytes: u64,
}
impl RasterCache {
    pub fn new(budget: u64) -> Self {
        Self {
            entries: Default::default(),
            budget,
            bytes: 0,
        }
    }
    pub fn get(&mut self, key: RenderKey) -> Option<(Arc<String>, bool)> {
        let index = self.entries.iter().position(|entry| entry.key == key)?;
        let entry = self.entries.remove(index)?;
        let result = (entry.source.clone(), entry.reduced);
        self.entries.push_back(entry);
        Some(result)
    }
    pub fn insert(&mut self, key: RenderKey, source: Arc<String>, bytes: u64, reduced: bool) {
        if let Some(index) = self.entries.iter().position(|entry| entry.key == key) {
            if let Some(entry) = self.entries.remove(index) {
                self.bytes -= entry.bytes;
            }
        }
        if bytes > self.budget {
            return;
        }
        while self.bytes > self.budget - bytes {
            if let Some(entry) = self.entries.pop_front() {
                self.bytes -= entry.bytes;
            } else {
                break;
            }
        }
        self.entries.push_back(Entry {
            key,
            source,
            bytes,
            reduced,
        });
        self.bytes += bytes;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn key(page: usize) -> RenderKey {
        RenderKey::new(page, 1., false)
    }
    fn source(value: &str) -> Arc<String> {
        Arc::new(value.to_owned())
    }

    #[test]
    fn event_source_survives_eviction_and_cache_drop() {
        let mut cache = RasterCache::new(10);
        let raster = source("data:image/png;base64,first");
        cache.insert(key(0), raster.clone(), 10, false);
        let event_source = cache.get(key(0)).unwrap().0;
        cache.insert(key(1), source("second"), 10, false);
        assert!(cache.get(key(0)).is_none());
        assert_eq!(*event_source, *raster);
        drop(cache);
        assert_eq!(event_source.as_str(), "data:image/png;base64,first");
    }

    #[test]
    fn decoded_and_encoded_budget_evicts_least_recently_used() {
        let mut cache = RasterCache::new(20);
        cache.insert(key(0), source("zero"), 10, false);
        cache.insert(key(1), source("one"), 10, false);
        assert!(cache.get(key(0)).is_some());
        cache.insert(key(2), source("two"), 10, true);
        assert!(cache.get(key(1)).is_none());
        assert!(cache.get(key(0)).is_some());
        assert!(cache.get(key(2)).unwrap().1);
        assert_eq!(cache.bytes, 20);
    }

    #[test]
    fn oversized_source_stays_owned_by_event_without_cache_retention() {
        let mut cache = RasterCache::new(10);
        let event_source = source("oversized");
        cache.insert(key(0), event_source.clone(), 20, false);
        assert!(cache.get(key(0)).is_none());
        assert_eq!(cache.bytes, 0);
        drop(cache);
        assert_eq!(event_source.as_str(), "oversized");
    }

    #[test]
    fn replacing_key_updates_budget_without_invalidating_old_event() {
        let mut cache = RasterCache::new(20);
        cache.insert(key(0), source("old"), 15, false);
        let old_event = cache.get(key(0)).unwrap().0;
        cache.insert(key(0), source("new"), 5, true);
        assert_eq!(cache.bytes, 5);
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.get(key(0)).unwrap().0.as_str(), "new");
        assert_eq!(old_event.as_str(), "old");
    }
}
