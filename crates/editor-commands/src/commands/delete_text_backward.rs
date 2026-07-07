use editor_model::ChildView;
use editor_resource::Resource;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::{
    apply_carry_from_selection, capture_first_charlike_paint, find_ancestor_textblock,
};
use crate::{CommandError, CommandResult};

pub fn delete_text_backward(tr: &mut Transaction, resource: &Resource) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;

    let (prev_offset, captured) = {
        let view = tr.state().view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;

        if pos.offset == 0 {
            return Ok(false);
        }

        let Some(ChildView::Leaf(leaf)) = node.child_at(pos.offset - 1) else {
            return Ok(false);
        };
        if leaf.as_char().is_none() {
            return Ok(false);
        }

        let captured = find_ancestor_textblock(&view, pos.node)
            .map(|block| capture_first_charlike_paint(tr.state(), block));

        let prev_offset = pos
            .resolve(&view)
            .and_then(|r| r.prev_grapheme(resource))
            .map(|r| r.offset())
            .unwrap_or(pos.offset - 1);

        (prev_offset, captured)
    };

    let delete_count = pos.offset - prev_offset;
    tr.remove_text(pos.node, prev_offset, delete_count)?;
    tr.set_selection(Some(Selection::collapsed(Position {
        node: pos.node,
        offset: prev_offset,
        affinity: Affinity::Upstream,
    })))?;

    if let Some(captured) = &captured {
        apply_carry_from_selection(tr, captured)?;
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
        transact_fail!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
    }

    #[test]
    fn delete_in_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Helo") } } }
            selection: (p1, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_at_end_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hell") } } }
            selection: (p1, 4, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_text_after_tab() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") tab } } }
            selection: (p1, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_at_start_of_text_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
    }

    #[test]
    fn delete_char_before_offset_five() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("HelloWorld") } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("HellWorld") } } }
            selection: (p1, 4, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_at_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
    }

    #[test]
    fn delete_unicode_char() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("한글") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("한") } } }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_last_char_records_first_charlike_paint() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { text("A") [bold] } } }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let dot = p1;
        let carry = actual.projected.carry_modifiers(dot);
        assert!(carry.values().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn delete_non_last_char_no_carry_change() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { text("Hi") [bold] } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let dot = p1;
        assert!(actual.projected.carry_modifiers(dot).is_empty());
    }

    #[test]
    fn backspace_to_empty_equals_range_delete_carry() {
        let (bs_initial, p1, ..) = state! {
            doc { root { p1: paragraph { text("A") [font_size(1600)] text("B") [italic] } } }
            selection: (p1, 2)
        };
        let mut cur = bs_initial;
        loop {
            let mut tr = editor_transaction::Transaction::new(&cur);
            let changed = delete_text_backward(&mut tr, &Resource::new_test()).unwrap();
            cur = tr.commit().0;
            if !changed {
                break;
            }
            let empty = cur
                .view()
                .node(p1)
                .map(|n| n.children().count() == 0)
                .unwrap_or(true);
            if empty {
                break;
            }
        }
        let bs_carry = cur.projected.carry_modifiers(p1);

        let (rng_initial, rp1, ..) = state! {
            doc { root { p1: paragraph { text("A") [font_size(1600)] text("B") [italic] } } }
            selection: (p1, 0) -> (p1, 2)
        };
        let (rng, ..) = transact!(rng_initial, |tr| crate::delete_selection(&mut tr));
        let rng_carry = rng.projected.carry_modifiers(rp1);

        assert_eq!(
            bs_carry, rng_carry,
            "backspace-to-empty and range-delete carry must match"
        );
        assert!(
            bs_carry
                .values()
                .any(|m| matches!(m, Modifier::FontSize { value: 1600 }))
        );
    }

    #[test]
    fn emptying_fold_title_replaces_carry() {
        let (initial, ft) = state! {
            doc { root { fold {
                ft: fold_title carry([italic]) { text("x") }
                fold_content { paragraph {} }
            } } }
            selection: (ft, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        assert!(
            actual.projected.carry_modifiers(ft).is_empty(),
            "emptying a fold title replaces its stale carry with the (empty) first charlike paint, got {:?}",
            actual.projected.carry_modifiers(ft)
        );
    }
}
