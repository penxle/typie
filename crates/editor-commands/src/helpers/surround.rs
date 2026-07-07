use editor_model::ChildView;
use editor_state::{Position, Selection};
use editor_state::{first_cursor_position, last_cursor_position};
use editor_transaction::Transaction;

use crate::helpers::{apply_inline_modifiers, child_leaf_dots, resolve_effective_modifiers};
use crate::{CommandError, CommandResult};

pub(crate) fn surround_selection(tr: &mut Transaction, left: &str, right: &str) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor == selection.head {
        return Ok(false);
    }

    let (from_pos, to_pos) = {
        let view = tr.view();
        let resolved = selection
            .resolve(&view)
            .ok_or_else(|| CommandError::Corrupted("cannot resolve selection".into()))?;

        let raw_from_id = resolved.from().node();
        let raw_from_offset = resolved.from().offset();
        let raw_from_affinity = resolved.from().affinity();
        let raw_to_id = resolved.to().node();
        let raw_to_offset = resolved.to().offset();
        let raw_to_affinity = resolved.to().affinity();

        let from_pos = {
            let node = view
                .node(raw_from_id)
                .ok_or(CommandError::NodeNotFound(raw_from_id))?;
            if node.spec().is_textblock() {
                Position {
                    node: raw_from_id,
                    offset: raw_from_offset,
                    affinity: raw_from_affinity,
                }
            } else {
                let child = match node.child_at(raw_from_offset) {
                    Some(ChildView::Block(b)) => first_cursor_position(&b),
                    _ => None,
                };
                match child {
                    Some(pos) if view.node(pos.node).is_some_and(|n| n.spec().is_textblock()) => {
                        pos
                    }
                    _ => return Ok(false),
                }
            }
        };

        let to_pos = {
            let node = view
                .node(raw_to_id)
                .ok_or(CommandError::NodeNotFound(raw_to_id))?;
            if node.spec().is_textblock() {
                Position {
                    node: raw_to_id,
                    offset: raw_to_offset,
                    affinity: raw_to_affinity,
                }
            } else {
                let child = raw_to_offset
                    .checked_sub(1)
                    .and_then(|idx| node.child_at(idx))
                    .and_then(|c| match c {
                        ChildView::Block(b) => last_cursor_position(&b),
                        _ => None,
                    });
                match child {
                    Some(pos) if view.node(pos.node).is_some_and(|n| n.spec().is_textblock()) => {
                        pos
                    }
                    _ => return Ok(false),
                }
            }
        };

        (from_pos, to_pos)
    };

    let from_id = from_pos.node;
    let to_id = to_pos.node;
    let from_offset = from_pos.offset;
    let to_offset = to_pos.offset;

    let left_char_count = left.chars().count();
    let right_char_count = right.chars().count();

    let left_paint = resolve_effective_modifiers(
        &tr.state().projected,
        from_id,
        from_offset,
        tr.pending_modifiers(),
    );
    let right_paint = resolve_effective_modifiers(
        &tr.state().projected,
        to_id,
        to_offset,
        tr.pending_modifiers(),
    );

    tr.insert_text(to_id, to_offset, right)?;
    let right_dots = child_leaf_dots(tr, to_id, to_offset, right_char_count);
    tr.insert_text(from_id, from_offset, left)?;
    let left_dots = child_leaf_dots(tr, from_id, from_offset, left_char_count);

    apply_inline_modifiers(tr, &left_dots, &left_paint)?;
    apply_inline_modifiers(tr, &right_dots, &right_paint)?;

    let new_to_offset = if from_id == to_id {
        to_offset + left_char_count + right_char_count
    } else {
        to_offset + right_char_count
    };

    let new_from = Position {
        node: from_id,
        offset: from_offset,
        affinity: from_pos.affinity,
    };
    let new_to = Position {
        node: to_id,
        offset: new_to_offset,
        affinity: to_pos.affinity,
    };
    tr.set_selection(Some(Selection::new(new_from, new_to)))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        transact_fail!(initial, |tr| surround_selection(&mut tr, "(", ")"));
    }

    #[test]
    fn surrounds_within_single_text_node() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6) -> (p1, 11)
        };
        let (actual, ..) = transact!(initial, |tr| surround_selection(&mut tr, "(", ")"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello (world)") } } }
            selection: (p1, 6) -> (p1, 13)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn surrounds_across_two_paragraphs() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("hello") }
                p2: paragraph { text("world") }
            } }
            selection: (p1, 2) -> (p2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| surround_selection(&mut tr, "[", "]"));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("he[llo") }
                p2: paragraph { text("wor]ld") }
            } }
            selection: (p1, 2) -> (p2, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn surrounds_block_unit_selection() {
        // Unit selection: the whole paragraph is selected at the root container level.
        // surround_selection must descend into the block and find the text endpoints.
        let (initial, _r1, ..) = state! {
            doc { r1: root { p1: paragraph { text("A") } } }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| surround_selection(&mut tr, "(", ")"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("(A)") } } }
            selection: (p1, 0) -> (p1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn ascii_quote_maps_to_curly_pair() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| surround_selection(
            &mut tr, "\u{201C}", "\u{201D}"
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("\u{201C}hello\u{201D}") } } }
            selection: (p1, 0) -> (p1, 7)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn surrounds_bold_range_paints_brackets_bold() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("가나") [bold] } } }
            selection: (p1, 0) -> (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| surround_selection(&mut tr, "(", ")"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("(가나)") [bold] } } }
            selection: (p1, 0) -> (p1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn surrounds_paints_each_bracket_by_its_own_position() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("가") [bold] text("나") } } }
            selection: (p1, 0) -> (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| surround_selection(&mut tr, "(", ")"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph {
                text("(가") [bold]
                text("나)")
            } } }
            selection: (p1, 0) -> (p1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn surrounds_inside_uniform_bold_run_keeps_neighbors_bold() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("가나다") [bold] } } }
            selection: (p1, 1) -> (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| surround_selection(&mut tr, "(", ")"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("가(나)다") [bold] } } }
            selection: (p1, 1) -> (p1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn surround_over_range_selection_is_pending_neutral() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("가나") [bold] } } }
            selection: (p1, 0) -> (p1, 2)
        };
        assert!(initial.pending_modifiers.is_empty());
        let (actual, ..) = transact!(initial, |tr| surround_selection(&mut tr, "(", ")"));
        assert!(actual.pending_modifiers.is_empty());
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("(가나)") [bold] } } }
            selection: (p1, 0) -> (p1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }
}
