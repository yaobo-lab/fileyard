use moka_cache::{moka::notification::RemovalCause, moka::sync::Cache, MokaCache, MokaCacheData};
use std::sync::Arc;
use std::time::Duration;

pub type CacheEvictionListener =
    Box<dyn Fn(Arc<String>, MokaCacheData, RemovalCause) + Send + Sync + 'static>;

pub fn new(
    cache_max_cap: u64,
    cache_time_to_idle: u64,
    cache_eviction_listener: Option<CacheEvictionListener>,
) -> MokaCache {
    //cache
    let mut c = Cache::builder()
        .max_capacity(cache_max_cap)
        .time_to_idle(Duration::from_millis(cache_time_to_idle));
    if let Some(eviction_f) = cache_eviction_listener {
        c = c.eviction_listener(eviction_f);
    }
    MokaCache(c.build())
}
