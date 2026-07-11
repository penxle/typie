editor_macros::preamble!();

mod affinity;
mod apply;
mod bind;
mod builders;
mod carry;
mod cell_selection;
mod classify;
mod composition;
mod continuation;
mod edit_commands;
mod error;
mod flat;
mod fragment_builder;
mod gap_cursor;
mod layout_dirty;
mod load_builder;
mod modifier_resolution;
mod modifier_span;
mod modifier_state;
mod normalize;
mod paragraph_break;
mod pending_modifier;
mod position;
mod projected_state;
mod prose;
mod replacement;
mod selection;
mod selection_expansion;
mod stable_position;
mod stable_selection;
mod state;
#[cfg(any(test, feature = "test-utils"))]
#[doc(hidden)]
pub mod test_utils;
mod to_plain;
mod traversal;
pub mod undo;

pub use affinity::*;
pub use apply::*;
pub use bind::*;
pub use builders::{
    cell_rect_selection, gap_cursor_selection_between, gap_cursor_selection_leading,
};
pub use carry::{block_accepts_carry_kind, end_touched_textblocks};
pub use cell_selection::{
    CellRect, as_cell_rect, enclosing_table, enclosing_table_cell, selected_table_cell_ids,
    table_cell_ids,
};
pub use composition::*;
pub use continuation::{
    apply_pending, caret_provided_and_override, continuation_at, continuation_from_neighbors,
};
pub use error::*;
pub use flat::{
    FLAT_CLOSE, FLAT_OPEN, FlatSegment, ResolvedPositionFlatExt, flat_chars, flat_segments,
    flat_segments_in_range, flat_segments_in_range_with_pos, flat_size, flat_text,
};
pub use gap_cursor::{GapCursor, as_gap_cursor, gap_cursor_at};
pub use layout_dirty::LayoutDirty;
pub use load_builder::BuildError;
pub use modifier_resolution::resolve_effective_modifiers_at;
pub use modifier_span::resolve_modifier_span_selection;
pub use modifier_state::{resolve_modifier_state, resolve_modifier_state_in_range};
pub use normalize::{doc_start_selection, farther_endpoint, is_unit_node_selection};
pub use paragraph_break::{
    before_or_same, closest_empty_paragraph_break_end_between, paragraph_break_at_end,
};
pub use pending_modifier::*;
pub use position::{Position, ResolvedPosition, inline_leaf_dots_in_range};
pub use projected_state::*;
pub use prose::{ProseText, prose};
pub use replacement::replacement_paint;
pub use selection::{ResolvedSelection, Selection};
pub use selection_expansion::{
    resolve_paragraph_selection_expansion, resolve_sentence_selection_expansion,
    resolve_word_selection_expansion,
};
pub use stable_position::{StablePosition, StablePositionChild, StableResolveCtx};
pub use stable_selection::StableSelection;
pub use state::*;
pub use to_plain::to_plain;
pub use traversal::{
    LeafGroup, blocks_in_range, first_cursor_position, last_cursor_position, leaf_groups_in_range,
    leaf_spans_in_range, leaves_in_block_range,
};
