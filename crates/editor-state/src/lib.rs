editor_macros::preamble!();

mod affinity;
mod apply;
mod bind;
mod cell_selection;
mod composition;
mod cursor_position;
mod error;
mod flat;
mod gap_cursor;
mod modifier_resolution;
mod normalize;
mod paragraph_break;
mod pending_modifier;
mod pending_style;
mod position;
mod prose;
mod resolved_position;
mod resolved_selection;
mod selection;
mod selection_expansion;
mod stable_position;
mod stable_selection;
mod state;

pub use affinity::*;
pub use apply::*;
pub use bind::*;
pub use cell_selection::*;
pub use composition::*;
pub use cursor_position::*;
pub use error::*;
pub use flat::*;
pub use gap_cursor::*;
pub use modifier_resolution::*;
pub use normalize::farther_endpoint;
pub use paragraph_break::{
    closest_empty_paragraph_break_end_between, paragraph_break_selection_at_paragraph_end,
};
pub use pending_modifier::*;
pub use pending_style::*;
pub use position::*;
pub use prose::*;
pub use resolved_position::*;
pub use resolved_selection::*;
pub use selection::*;
pub use selection_expansion::*;
pub use stable_position::*;
pub use stable_selection::*;
pub use state::*;

#[cfg(any(test, feature = "test-utils"))]
mod test_utils;

#[cfg(any(test, feature = "test-utils"))]
pub use test_utils::*;
