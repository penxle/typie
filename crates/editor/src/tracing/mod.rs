mod collector;
mod data;
mod reporter;
mod span;
mod tracer;

pub use reporter::TracingReporter;
pub(crate) use tracer::TracingTracer;

use collector::TracingCollector;
use std::cell::RefCell;

thread_local! {
    pub(crate) static TRACING_COLLECTOR: RefCell<Option<TracingCollector>> = const { RefCell::new(None) };
}

pub(crate) const TRACER: TracingTracer = TracingTracer;

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
pub(crate) fn now() -> std::time::SystemTime {
    let epoch_ms = web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.time_origin() + p.now())
        .unwrap_or_else(|| js_sys::Date::now());
    let duration = std::time::Duration::from_secs_f64(epoch_ms / 1000.0);
    std::time::SystemTime::UNIX_EPOCH + duration
}

#[cfg(not(all(feature = "wasm", target_arch = "wasm32")))]
pub(crate) fn now() -> std::time::SystemTime {
    std::time::SystemTime::now()
}
