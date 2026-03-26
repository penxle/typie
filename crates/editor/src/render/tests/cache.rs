use super::*;

#[test]
fn prune_page_cache_removes_entries_outside_page_count() {
    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());

    renderer.page_cache.insert(0, PageCache::new(64, 64, 1.0));
    renderer.page_cache.insert(1, PageCache::new(64, 64, 1.0));
    renderer.page_cache.insert(3, PageCache::new(64, 64, 1.0));

    renderer.prune_page_cache(2);

    assert_eq!(renderer.page_cache.len(), 2);
    assert!(renderer.page_cache.contains_key(&0));
    assert!(renderer.page_cache.contains_key(&1));
    assert!(!renderer.page_cache.contains_key(&3));
}
