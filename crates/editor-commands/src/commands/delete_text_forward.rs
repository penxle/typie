use editor_model::ChildView;
use editor_resource::Resource;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::{
    apply_first_text_marker_lift, capture_first_text_marker, find_enclosing_paragraph_id,
};
use crate::{CommandError, CommandResult};

pub fn delete_text_forward(tr: &mut Transaction, resource: &Resource) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;

    let (delete_count, captured) = {
        let view = tr.state().view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;

        let Some(ChildView::Leaf(leaf)) = node.child_at(pos.offset) else {
            return Ok(false);
        };
        if leaf.as_char().is_none() {
            return Ok(false);
        }

        let captured = find_enclosing_paragraph_id(&view, pos.node)
            .and_then(|id| capture_first_text_marker(tr.state(), id));

        let next_offset = pos
            .resolve(&view)
            .and_then(|r| r.next_grapheme(resource))
            .map(|r| r.offset())
            .unwrap_or(pos.offset + 1);

        (next_offset - pos.offset, captured)
    };

    tr.remove_text(pos.node, pos.offset, delete_count)?;
    tr.set_selection(Some(Selection::collapsed(Position {
        node: pos.node,
        offset: pos.offset,
        affinity: Affinity::Downstream,
    })))?;

    if let Some(captured) = captured {
        apply_first_text_marker_lift(tr, &captured)?;
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;
    use editor_resource::Resource;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 3)
        };
        transact_fail!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
    }

    #[test]
    fn delete_at_start_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ello") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_in_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Helo") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_text_after_tab() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") tab } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_at_end_of_text_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
        };
        transact_fail!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
    }

    #[test]
    fn delete_char_at_offset_five() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("HelloWorld") } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Helloorld") } } }
            selection: (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_at_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
    }

    #[test]
    fn delete_unicode_char() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("한글") } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("글") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_last_char_forward_lifts_marker() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { text("A") [bold] } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
        let dot = p1;
        let marker = actual
            .projected
            .node_markers()
            .value_of(dot)
            .expect("paragraph should have a marker");
        assert!(marker.modifiers.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn delete_non_last_char_forward_no_lift() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { text("Hi") [bold] } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
        let dot = p1;
        assert!(actual.projected.node_markers().value_of(dot).is_none());
    }
}
