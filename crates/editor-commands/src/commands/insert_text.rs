use editor_common::StrExt;
use editor_model::{Node, NodeId, Subtree, TextNode};
use editor_state::{Affinity, PendingModifiers, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::resolve_effective_modifiers;
use crate::{CommandError, CommandResult};

pub fn insert_text(tr: &mut Transaction, text: &str) -> CommandResult {
    if text.is_empty() {
        return Err(CommandError::InvalidArgument(
            "text must not be empty".into(),
        ));
    }

    if text.contains(['\n', '\r']) {
        return Err(CommandError::InvalidArgument(
            "text must not contain newlines".into(),
        ));
    }

    let selection = tr.selection();
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();

    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let effective_mods = resolve_effective_modifiers(&node, pos.offset, tr.pending_modifiers());
    let insert_len = text.char_count();

    match node.node() {
        Node::Text(text_node) => {
            if effective_mods == node.modifiers() {
                tr.insert_text(pos.node_id, pos.offset, text)?;
                tr.set_selection(Selection::collapsed(Position {
                    node_id: pos.node_id,
                    offset: pos.offset + insert_len,
                    affinity: Affinity::Upstream,
                }))?;
            } else {
                let parent = node.parent().ok_or(CommandError::NoParent(pos.node_id))?;
                let node_index = node
                    .index()
                    .ok_or(CommandError::orphan_child(pos.node_id, parent.id()))?;

                let new_id = NodeId::new();
                let subtree = Subtree::leaf(new_id, Node::Text(TextNode { text: text.into() }))
                    .with_modifiers(effective_mods);

                if pos.offset == 0 {
                    tr.insert_subtree(parent.id(), node_index, subtree)?;
                } else if pos.offset == text_node.text.char_count() {
                    tr.insert_subtree(parent.id(), node_index + 1, subtree)?;
                } else {
                    let split_id = NodeId::new();
                    tr.split_node(pos.node_id, pos.offset, split_id)?;
                    tr.insert_subtree(parent.id(), node_index + 1, subtree)?;
                }

                tr.set_selection(Selection::collapsed(Position {
                    node_id: new_id,
                    offset: insert_len,
                    affinity: Affinity::Upstream,
                }))?;
            }
        }
        _ => {
            // Case 3: non-text node (empty paragraph, etc.)
            let new_id = NodeId::new();
            let subtree = Subtree::leaf(new_id, Node::Text(TextNode { text: text.into() }))
                .with_modifiers(effective_mods);

            tr.insert_subtree(pos.node_id, pos.offset, subtree)?;
            tr.set_selection(Selection::collapsed(Position {
                node_id: new_id,
                offset: insert_len,
                affinity: Affinity::Upstream,
            }))?;
        }
    }

    if !tr.pending_modifiers().is_empty() {
        tr.set_pending_modifiers(PendingModifiers::new())?;
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::CommandError;
    use crate::test_utils::*;

    #[test]
    fn empty_text_returns_error() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let err = transact_err!(initial, |tr| insert_text(&mut tr, ""));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn newline_returns_error() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let err = transact_err!(initial, |tr| insert_text(&mut tr, "a\nb"));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn carriage_return_returns_error() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let err = transact_err!(initial, |tr| insert_text(&mut tr, "a\rb"));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 3)
        };
        transact_fail!(initial, |tr| insert_text(&mut tr, "X"));
    }

    #[test]
    fn insert_into_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "XY"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("HeXYllo") } } }
            selection: (t1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_start_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "AB"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ABHello") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "!"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello!") } } }
            selection: (t1, 6)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_unicode_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "한글"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello한글") } } }
            selection: (t1, 7)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_with_pending_bold_creates_new_node() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("X") [bold]
                    }
                }
            }
            selection: (t2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_start_with_different_mods_creates_node_before() {
        // Bold has Expand::After → not inherited at start → effective = []
        // Current mods = [Bold] → mismatch → new node before
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("X")
                        t2: text("Hello") [bold]
                    }
                }
            }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_in_middle_with_pending_splits_node() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        // "He" [] → "X" [Bold] → "llo" []
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("He")
                        t2: text("X") [bold]
                        t3: text("llo")
                    }
                }
            }
            selection: (t2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_link_creates_node_after() {
        // Link has Expand::None → not inherited → new node after
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Click") [link(href: "https://a.com".to_string())] } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, " here"));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Click") [link(href: "https://a.com".to_string())]
                        t2: text(" here")
                    }
                }
            }
            selection: (t2, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_bold_stays_inline() {
        // Bold has Expand::After → inherited at end → match → Case 1
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "!"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello!") [bold] } } }
            selection: (t1, 6)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_into_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "Hello"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pending_modifiers_cleared_after_insert() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        assert!(actual.pending_modifiers.is_empty());
    }

    #[test]
    fn insert_into_non_textblock_returns_error() {
        let (initial, ..) = state! {
            doc { root { hr: horizontal_rule {} } }
            selection: (hr, 0)
        };
        let err = transact_err!(initial, |tr| insert_text(&mut tr, "X"));
        assert!(matches!(err, CommandError::Step(_)));
    }

    #[test]
    fn pending_unset_on_bold_text_creates_new_node() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 5)
            pending_modifiers: [!bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello") [bold]
                        t2: text("X")
                    }
                }
            }
            selection: (t2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }
}
