mod list;
mod list_kind;
mod selection;
mod slice;

pub use list::{judge_indent_list, judge_outdent_list};
pub use list_kind::judge_toggle_list_kind;
pub use selection::{
    judge_expand_all, judge_expand_paragraph, judge_expand_sentence, judge_expand_word,
};
pub use slice::{SliceInsertionPlan, resolve_slice_insertion};

pub(crate) use list::{lift_selected_list_items, sink_selected_list_items};
pub(crate) use list_kind::{judge_lift_list_items_of_kind, judge_set_list_kind};
pub(crate) use slice::insert_slice_at_position;
