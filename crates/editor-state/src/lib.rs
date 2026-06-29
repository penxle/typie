editor_macros::preamble!();

mod affinity;
mod apply;
mod bind;
mod builders;
mod cell_selection;
mod classify;
mod composition;
mod edit_commands;
mod error;
mod flat;
mod fragment_builder;
mod gap_cursor;
mod load_builder;
mod modifier_resolution;
mod modifier_span;
mod modifier_state;
mod normalize;
mod paragraph_break;
mod pending_modifier;
mod pending_style;
mod position;
mod projected_state;
mod prose;
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
pub use cell_selection::{
    CellRect, as_cell_rect, as_node_selection, enclosing_table, enclosing_table_cell,
    table_cell_ids,
};
pub use composition::*;
pub use error::*;
pub use flat::{
    FLAT_CLOSE, FLAT_OPEN, FlatSegment, ResolvedPositionFlatExt, flat_chars, flat_segments,
    flat_segments_in_range, flat_size, flat_text,
};
pub use gap_cursor::{GapCursor, as_gap_cursor, gap_cursor_at};
pub use load_builder::BuildError;
pub use modifier_span::resolve_modifier_span_selection;
pub use modifier_state::{resolve_modifier_state, resolve_modifier_state_in_range};
pub use normalize::{doc_start_selection, farther_endpoint, is_unit_node_selection};
pub use paragraph_break::{
    before_or_same, closest_empty_paragraph_break_end_between, paragraph_break_at_end,
};
pub use pending_modifier::*;
pub use pending_style::*;
pub use position::{Position, ResolvedPosition, inline_leaf_dots_in_range};
pub use projected_state::*;
pub use prose::{ProseText, prose};
pub use selection::{ResolvedSelection, Selection};
pub use selection_expansion::{
    resolve_paragraph_selection_expansion, resolve_sentence_selection_expansion,
    resolve_word_selection_expansion,
};
pub use stable_position::{StablePosition, StableResolveCtx};
pub use stable_selection::{StableSelection, resolve_effective_modifiers_at};
pub use state::*;
pub use to_plain::to_plain;
pub use traversal::{first_cursor_position, last_cursor_position};
