#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
pub(super) type ProfileInstant = f64;

#[cfg(not(all(feature = "wasm", target_arch = "wasm32")))]
pub(super) type ProfileInstant = std::time::Instant;

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
pub(super) fn profile_now() -> ProfileInstant {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now())
        .unwrap_or_else(js_sys::Date::now)
}

#[cfg(not(all(feature = "wasm", target_arch = "wasm32")))]
pub(super) fn profile_now() -> ProfileInstant {
    std::time::Instant::now()
}

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
pub(super) fn profile_elapsed_ms(started_at: ProfileInstant) -> f64 {
    (profile_now() - started_at).max(0.0)
}

#[cfg(not(all(feature = "wasm", target_arch = "wasm32")))]
pub(super) fn profile_elapsed_ms(started_at: ProfileInstant) -> f64 {
    started_at.elapsed().as_secs_f64() * 1000.0
}
