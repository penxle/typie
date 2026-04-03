pub(crate) mod cursor;
pub(crate) mod hit_test;
pub(crate) mod navigation;
pub(crate) mod search;
pub(crate) mod segmentation;
mod visit;

pub(crate) use cursor::cursor_rect;
pub(crate) use hit_test::{closest_hit_test, exact_hit_test};
pub(crate) use navigation::resolve_movement;
pub use visit::{Edges, PageVisitor, visit_page};
