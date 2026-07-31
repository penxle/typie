use editor_crdt::Dot;
use editor_model::{Fragment, NodeType};
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::insert_terminal_page_break_into_root_paragraph;

pub(super) fn insert_terminal_page_break_from_edge(
    tr: &mut Transaction,
    paragraph_id: Dot,
    fragments: &[Fragment],
) -> CommandResult {
    if !fragments.last().is_some_and(is_page_break_fragment) {
        return Ok(false);
    }
    insert_terminal_page_break_into_root_paragraph(tr, paragraph_id)
}

pub(super) fn paragraph_ends_with_page_break(fragment: &Fragment) -> bool {
    fragment.node.as_type() == NodeType::Paragraph
        && fragment.children.last().is_some_and(is_page_break_fragment)
}

fn is_page_break_fragment(fragment: &Fragment) -> bool {
    fragment.node.as_type() == NodeType::PageBreak
}
