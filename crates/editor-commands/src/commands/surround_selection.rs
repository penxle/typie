use editor_model::Node;
use editor_state::{NodeRefCursorExt, Position, Selection};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn surround_selection(tr: &mut Transaction, left: &str, right: &str) -> CommandResult {
    let selection = tr.selection();
    if selection.is_collapsed() {
        return Ok(false);
    }

    let doc = tr.doc();
    let resolved = selection
        .resolve(&doc)
        .ok_or_else(|| CommandError::Corrupted("cannot resolve selection".into()))?;

    let raw_from_id = resolved.from().node_id();
    let raw_from_offset = resolved.from().offset();
    let raw_to_id = resolved.to().node_id();
    let raw_to_offset = resolved.to().offset();

    let from_pos = {
        let doc = tr.doc();
        let node = doc
            .node(raw_from_id)
            .ok_or(CommandError::NodeNotFound(raw_from_id))?;
        if matches!(node.node(), Node::Text(_)) {
            Position {
                node_id: raw_from_id,
                offset: raw_from_offset,
                affinity: resolved.from().affinity(),
            }
        } else {
            let child = node
                .children()
                .nth(raw_from_offset)
                .and_then(|c| c.first_cursor_position());
            match child {
                Some(pos)
                    if doc
                        .node(pos.node_id)
                        .is_some_and(|n| matches!(n.node(), Node::Text(_))) =>
                {
                    pos
                }
                _ => return Ok(false),
            }
        }
    };

    let to_pos = {
        let doc = tr.doc();
        let node = doc
            .node(raw_to_id)
            .ok_or(CommandError::NodeNotFound(raw_to_id))?;
        if matches!(node.node(), Node::Text(_)) {
            Position {
                node_id: raw_to_id,
                offset: raw_to_offset,
                affinity: resolved.to().affinity(),
            }
        } else {
            let child = raw_to_offset
                .checked_sub(1)
                .and_then(|idx| node.children().nth(idx))
                .and_then(|c| c.last_cursor_position());
            match child {
                Some(pos)
                    if doc
                        .node(pos.node_id)
                        .is_some_and(|n| matches!(n.node(), Node::Text(_))) =>
                {
                    pos
                }
                _ => return Ok(false),
            }
        }
    };

    let from_id = from_pos.node_id;
    let to_id = to_pos.node_id;
    let from_offset = from_pos.offset;
    let to_offset = to_pos.offset;

    let left_char_count = left.chars().count();
    let right_char_count = right.chars().count();

    // Insert right first so the from position is not shifted.
    tr.insert_text(to_id, to_offset, right)?;
    tr.insert_text(from_id, from_offset, left)?;

    let new_to_offset = if from_id == to_id {
        to_offset + left_char_count + right_char_count
    } else {
        to_offset + right_char_count
    };

    let new_from = Position {
        node_id: from_id,
        offset: from_offset,
        affinity: from_pos.affinity,
    };
    let new_to = Position {
        node_id: to_id,
        offset: new_to_offset,
        affinity: to_pos.affinity,
    };
    tr.set_selection(Selection::new(new_from, new_to))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::assert_state_eq;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        transact_fail!(initial, |tr| surround_selection(&mut tr, "(", ")"));
    }

    #[test]
    fn surrounds_within_single_text_node() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 6) -> (t1, 11)
        };
        let (actual, ..) = transact!(initial, |tr| surround_selection(&mut tr, "(", ")"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello (world)") } } }
            selection: (t1, 6) -> (t1, 13)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn surrounds_across_two_paragraphs() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("hello") }
                paragraph { t2: text("world") }
            } }
            selection: (t1, 2) -> (t2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| surround_selection(&mut tr, "[", "]"));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("he[llo") }
                paragraph { t2: text("wor]ld") }
            } }
            selection: (t1, 2) -> (t2, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn surrounds_block_unit_selection() {
        // Unit selection: the whole paragraph is selected at the root container level.
        // auto_surround must descend into the block and find the text endpoints.
        let (initial, _r1, ..) = state! {
            doc { r1: root { paragraph { t1: text("A") } } }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| surround_selection(&mut tr, "(", ")"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("(A)") } } }
            selection: (t1, 0) -> (t1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn ascii_quote_maps_to_curly_pair() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| surround_selection(
            &mut tr, "\u{201C}", "\u{201D}"
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("\u{201C}hello\u{201D}") } } }
            selection: (t1, 0) -> (t1, 7)
        };
        assert_state_eq!(&actual, &expected);
    }
}
