use editor_crdt::Dot;
use editor_model::ChildView;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::{remove_atom_leaf, remove_subtree_full};
use crate::{CommandError, CommandResult};

enum ForwardTarget {
    InlineAtom,
    Block(Dot),
}

pub fn delete_node_forward(tr: &mut Transaction) -> CommandResult {
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

        match node.child_at(pos.offset) {
            Some(ChildView::Leaf(l)) if l.as_char().is_none() => ForwardTarget::InlineAtom,
            Some(ChildView::Block(b)) => ForwardTarget::Block(b.id()),
            _ => return Ok(false),
        }
    };

    match target {
        ForwardTarget::InlineAtom => remove_atom_leaf(tr, pos.node, pos.offset)?,
        ForwardTarget::Block(child) => remove_subtree_full(tr, child)?,
    }

    tr.set_selection(Some(Selection::collapsed(Position {
        node: pos.node,
        offset: pos.offset,
        affinity: Affinity::Upstream,
    })))?;

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
        transact_fail!(initial, |tr| delete_node_forward(&mut tr));
    }

    #[test]
    fn delete_hard_break_after_text() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello")
                        hard_break
                        text("World")
                    }
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("HelloWorld")
                    }
                }
            }
            selection: (p1, 5, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_hard_break_at_paragraph_offset() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        hard_break
                        text("Hello")
                    }
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello")
                    }
                }
            }
            selection: (p1, 0, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn no_next_sibling_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
        };
        transact_fail!(initial, |tr| delete_node_forward(&mut tr));
    }

    #[test]
    fn next_is_text_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("HelloWorld") } } }
            selection: (p1, 5)
        };
        transact_fail!(initial, |tr| delete_node_forward(&mut tr));
    }

    #[test]
    fn in_middle_of_text_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
        };
        transact_fail!(initial, |tr| delete_node_forward(&mut tr));
    }

    #[test]
    fn at_paragraph_end_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 1)
        };
        transact_fail!(initial, |tr| delete_node_forward(&mut tr));
    }
}
