use editor_crdt::Dot;
use editor_model::ChildView;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::{
    apply_carry_from_selection, capture_first_charlike_paint, find_ancestor_textblock,
    remove_atom_leaf, remove_subtree_full,
};
use crate::{CommandError, CommandResult};

enum BackwardTarget {
    InlineAtom(usize),
    Block(Dot),
}

pub fn delete_node_backward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;

    let target = {
        let view = tr.state().view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;

        if pos.offset == 0 {
            return Ok(false);
        }

        match node.child_at(pos.offset - 1) {
            Some(ChildView::Leaf(l)) if l.as_char().is_none() => {
                BackwardTarget::InlineAtom(pos.offset - 1)
            }
            Some(ChildView::Block(b)) => BackwardTarget::Block(b.id()),
            _ => return Ok(false),
        }
    };

    let captured = match &target {
        BackwardTarget::InlineAtom(_) => {
            let view = tr.state().view();
            find_ancestor_textblock(&view, pos.node)
                .map(|block| capture_first_charlike_paint(tr.state(), block))
        }
        BackwardTarget::Block(_) => None,
    };

    match target {
        BackwardTarget::InlineAtom(idx) => remove_atom_leaf(tr, pos.node, idx)?,
        BackwardTarget::Block(child) => remove_subtree_full(tr, child)?,
    }

    tr.set_selection(Some(Selection::collapsed(Position {
        node: pos.node,
        offset: pos.offset - 1,
        affinity: Affinity::Downstream,
    })))?;

    if let Some(captured) = &captured {
        apply_carry_from_selection(tr, captured)?;
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 3)
        };
        transact_fail!(initial, |tr| delete_node_backward(&mut tr));
    }

    #[test]
    fn delete_hard_break_before_text() {
        let (initial, ..) = state! {
            doc {
                root {
                    p: paragraph {
                        text("Hello")
                        hard_break
                        text("World")
                    }
                }
            }
            selection: (p, 6)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("HelloWorld")
                    }
                }
            }
            selection: (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_hard_break_at_paragraph_offset() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello")
                        hard_break
                    }
                }
            }
            selection: (p1, 6)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello")
                    }
                }
            }
            selection: (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_no_prev_sibling_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_node_backward(&mut tr));
    }

    #[test]
    fn delete_prev_text_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("HelloWorld") } } }
            selection: (p1, 5)
        };
        transact_fail!(initial, |tr| delete_node_backward(&mut tr));
    }

    #[test]
    fn delete_in_middle_of_text_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
        };
        transact_fail!(initial, |tr| delete_node_backward(&mut tr));
    }

    #[test]
    fn delete_at_paragraph_start_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_node_backward(&mut tr));
    }

    #[test]
    fn backspace_over_sole_tab_records_font_size_carry() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { tab [font_size(1600)] } } }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node_backward(&mut tr));
        let carry = actual.projected.carry_modifiers(p1);
        assert!(
            carry
                .values()
                .any(|m| matches!(m, editor_model::Modifier::FontSize { value: 1600 })),
            "got {carry:?}"
        );
    }
}
