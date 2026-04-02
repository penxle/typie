editor_macros::preamble!();

mod affinity;
mod composition;
mod pending_modifier;
mod position;
mod resolved_position;
mod resolved_selection;
mod selection;
mod state;

pub use affinity::*;
pub use composition::*;
pub use pending_modifier::*;
pub use position::*;
pub use resolved_position::*;
pub use resolved_selection::*;
pub use selection::*;
pub use state::*;

#[cfg(any(test, feature = "test-utils"))]
mod test_utils;

#[cfg(any(test, feature = "test-utils"))]
pub use test_utils::*;
