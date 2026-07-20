editor_macros::preamble!();

pub mod glyph_run;
pub mod style;

mod dnd;
mod external;
pub(crate) mod measure;
pub(crate) mod paginate;
pub(crate) mod query;
mod table_overlay;

mod page;
pub mod page_fragment;
mod view;
mod view_state;
mod viewport;

pub use dnd::*;
pub use external::ExternalElement;
pub use measure::text::measure::TabGap;
pub use measure::text::ruby::ruby_extra_top;
pub use page::*;
pub use page_fragment::{
    PageFragmentAtom, PageFragmentBox, PageFragmentContent, PageFragmentDecoration,
    PageFragmentLine, PageFragmentNode, PageFragmentTree,
};
pub use paginate::types::ChildAttachment;
pub use query::interactive::{InteractiveHit, InteractiveRegion};
pub use query::link::LinkRect;
pub use query::selection::SelectionEndpoints;
pub use query::*;
pub use table_overlay::TableOverlay;
pub use view::*;
pub use view_state::*;
pub use viewport::*;
