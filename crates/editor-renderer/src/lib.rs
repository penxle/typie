editor_macros::preamble!();

pub mod backend;
pub mod damage;
pub mod diff;
pub mod display_list;
pub(crate) mod glyph;
pub mod icon_data;
pub mod icons;
pub mod renderer;
pub mod sink;
pub mod types;
pub mod vector;

pub use backend::RenderBackend;
pub use renderer::{Mark, MarkData, MarkRect, Renderer};
pub use sink::RenderSink;
pub use types::*;
