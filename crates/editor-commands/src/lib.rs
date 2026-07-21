editor_macros::preamble!();

mod commands;
mod compose;
mod error;
pub(crate) mod helpers;
mod judgments;
pub mod types;

#[cfg(test)]
mod test_utils;

#[cfg(test)]
mod tests;

pub use commands::*;
pub use compose::*;
pub use error::*;
pub use judgments::{judge_indent_list, judge_outdent_list, judge_toggle_list_kind};
pub use types::ListVerdict;
