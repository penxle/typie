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
pub use judgments::{
    SliceInsertionPlan, judge_expand_all, judge_expand_paragraph, judge_expand_sentence,
    judge_expand_word, judge_indent_list, judge_outdent_list, judge_toggle_list_kind,
    resolve_slice_insertion,
};
pub use types::Verdict;
