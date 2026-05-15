editor_macros::preamble!();

mod affinity;
mod apply;
mod bind;
mod cell_selection;
mod composition;
mod cursor_position;
mod error;
mod flat;
mod modifier_resolution;
mod normalize;
mod pending_modifier;
mod position;
mod resolved_position;
mod resolved_selection;
mod selection;
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
pub use modifier_resolution::*;
pub use pending_modifier::*;
pub use position::*;
pub use resolved_position::*;
pub use resolved_selection::*;
pub use selection::*;
pub use stable_position::*;
pub use stable_selection::*;
pub use state::*;

#[cfg(any(test, feature = "test-utils"))]
mod test_utils;

#[cfg(any(test, feature = "test-utils"))]
pub use test_utils::*;
