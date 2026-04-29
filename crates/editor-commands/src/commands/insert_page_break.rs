use editor_common::StrExt;
use editor_model::{Node, NodeId, PageBreakNode, Subtree};
use editor_state::Selection;
use editor_transaction::Transaction;

use crate::commands::split_paragraph;
use crate::helpers::{find_ancestor_textblock, find_first_cursor_position};
use crate::{CommandError, CommandResult};

pub fn insert_page_break(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    if !selection.is_collapsed() {
        return Ok(false);
    }
    let sp = tr.savepoint();

    let pos = selection.head;
    let doc = tr.doc();
    let Some(paragraph_id) = find_ancestor_textblock(&doc, pos.node_id) else {
        return Ok(false);
    };
    let paragraph = doc
        .node(paragraph_id)
        .ok_or(CommandError::NodeNotFound(paragraph_id))?;
    if !matches!(paragraph.node(), Node::Paragraph(_))
        || paragraph
            .parent()
            .is_none_or(|parent| parent.id() != NodeId::ROOT)
    {
        return Ok(false);
    }

    let at_paragraph_end = if pos.node_id == paragraph_id {
        pos.offset == paragraph.entry().children.len()
    } else {
        let node = doc
            .node(pos.node_id)
            .ok_or(CommandError::NodeNotFound(pos.node_id))?;
        matches!(node.node(), Node::Text(text) if
            node.parent().is_some_and(|parent| parent.id() == paragraph_id)
                && node.next_sibling().is_none()
                && pos.offset == text.text.char_count()
        )
    };
    let next_cursor = paragraph
        .next_sibling()
        .and_then(|next| find_first_cursor_position(&next))
        .map(Selection::collapsed);
    let split_before_insert = !at_paragraph_end || next_cursor.is_none();

    let (target_paragraph_id, selection_after_insert) = if split_before_insert {
        if !split_paragraph(tr)? {
            tr.rollback(sp);
            return Ok(false);
        }

        let split_selection = tr.selection();
        let doc = tr.doc();
        let current_paragraph_id = find_ancestor_textblock(&doc, split_selection.head.node_id)
            .ok_or(CommandError::Corrupted("no textblock ancestor".into()))?;
        let current_paragraph = doc
            .node(current_paragraph_id)
            .ok_or(CommandError::NodeNotFound(current_paragraph_id))?;
        let previous = current_paragraph
            .prev_sibling()
            .ok_or(CommandError::Corrupted(
                "split paragraph has no previous sibling".into(),
            ))?;

        (previous.id(), Some(split_selection))
    } else {
        (paragraph_id, next_cursor)
    };

    let doc = tr.doc();
    let target_paragraph = doc
        .node(target_paragraph_id)
        .ok_or(CommandError::NodeNotFound(target_paragraph_id))?;
    if target_paragraph
        .children()
        .any(|child| matches!(child.node(), Node::PageBreak(_)))
    {
        tr.rollback(sp);
        return Ok(false);
    }

    let page_break_id = NodeId::new();
    tr.insert_subtree(
        target_paragraph_id,
        target_paragraph.entry().children.len(),
        Subtree::leaf(page_break_id, Node::PageBreak(PageBreakNode::default())),
    )?;
    if let Some(selection) = selection_after_insert {
        tr.set_selection(selection)?;
    }

    Ok(true)
}
