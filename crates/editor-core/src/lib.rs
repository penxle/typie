editor_macros::preamble!();

mod block_state;
mod editor;
mod error;
mod event;
mod font;
mod handle;
mod history;
mod ime;
mod interaction;
mod message;
mod search;
mod state_field;
mod tracked_range;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

#[cfg(test)]
mod text_replacement_tests;

#[cfg(test)]
mod tests;

pub use block_state::*;
pub use editor::*;
pub use error::*;
pub use event::*;
pub use handle::*;
pub use history::*;
pub use ime::*;
pub use message::*;
pub use search::find_matches;
pub use state_field::*;
pub use tracked_range::*;
