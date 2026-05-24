editor_macros::preamble!();

pub mod glyph_run;
pub mod style;

mod external;
pub(crate) mod measure;
pub(crate) mod paginate;
pub(crate) mod query;
mod table_overlay;

mod page;
mod view;
mod view_state;
mod viewport;

#[derive(Debug, Clone)]
pub struct TableLayoutInfo {
    pub col_inner_widths: Vec<f32>,
    pub row_inner_heights: Vec<f32>,
}

pub use external::*;
pub use page::*;
pub use query::*;
pub use table_overlay::*;
pub use view::*;
pub use view_state::*;
pub use viewport::*;
