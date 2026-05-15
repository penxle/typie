pub(crate) mod common;
pub(crate) mod composition;
pub(crate) mod cursor;
pub(crate) mod grapheme;
pub(crate) mod hit_test;
pub(crate) mod navigation;
pub(crate) mod search;
pub(crate) mod segmentation;
pub(crate) mod selection;
mod visit;

pub use composition::CompositionRect;
pub use cursor::CursorMetrics;
pub(crate) use cursor::cursor_metrics;
pub(crate) use hit_test::{closest_hit_test, exact_hit_test};
pub(crate) use navigation::resolve_movement;
pub use selection::{SelectionRect, SelectionRectKind};
pub use visit::{Edges, LineMetrics, PageVisitor, visit_page};
