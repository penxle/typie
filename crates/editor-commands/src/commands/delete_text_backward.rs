use editor_model::{Node, NodeId};
use editor_resource::Resource;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::{
    apply_first_text_marker_lift, capture_first_text_marker, find_enclosing_paragraph_id,
};
use crate::{CommandError, CommandResult};

pub fn delete_text_backward(tr: &mut Transaction, resource: &Resource) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let Node::Text(text_node) = node.node() else {
        return Ok(false);
    };

    let captured_paragraph_id = find_enclosing_paragraph_id(&doc, pos.node_id);
    let captured = captured_paragraph_id.and_then(|id| capture_first_text_marker(&doc, id));

    if pos.offset > 0 {
        let text_len = text_node.text.len();
        let doc = tr.doc();
        let prev_offset = pos
            .resolve(&doc)
            .and_then(|r| r.prev_grapheme(resource))
            .map(|r| r.offset())
            .unwrap_or(pos.offset.saturating_sub(1));
        let delete_count = pos.offset - prev_offset;
        let is_last_char = text_len == delete_count;

        if is_last_char {
            let parent_id = node
                .parent()
                .ok_or(CommandError::NoParent(pos.node_id))?
                .id();
            let node_index = node
                .index()
                .ok_or(CommandError::orphan_child(pos.node_id, parent_id))?;
            let prev_id = node.prev_sibling().map(|n| n.id());
            let next_id = node.next_sibling().map(|n| n.id());

            tr.remove_subtree(pos.node_id)?;

            let new_selection =
                resolve_cursor_after_removal(tr, prev_id, next_id, parent_id, node_index);
            tr.set_selection(Some(new_selection))?;
        } else {
            tr.remove_text(pos.node_id, prev_offset, delete_count)?;
            tr.set_selection(Some(Selection::collapsed(Position {
                node_id: pos.node_id,
                offset: prev_offset,
                affinity: Affinity::Upstream,
            })))?;
        }
    } else {
        // offset == 0: try deleting last char of previous text sibling
        let prev = match node.prev_sibling() {
            Some(prev) => prev,
            None => return Ok(false),
        };

        let Node::Text(prev_text) = prev.node() else {
            return Ok(false);
        };

        let prev_id = prev.id();
        let prev_len = prev_text.text.len();
        let is_last_char = prev_len == 1;

        if is_last_char {
            tr.remove_subtree(prev_id)?;
            tr.set_selection(Some(Selection::collapsed(Position {
                node_id: pos.node_id,
                offset: 0,
                affinity: Affinity::Downstream,
            })))?;
        } else {
            tr.remove_text(prev_id, prev_len - 1, 1)?;
            tr.set_selection(Some(Selection::collapsed(Position {
                node_id: prev_id,
                offset: prev_len - 1,
                affinity: Affinity::Upstream,
            })))?;
        }
    }

    if let Some(captured) = captured {
        apply_first_text_marker_lift(tr, &captured)?;
    }

    Ok(true)
}

fn resolve_cursor_after_removal(
    tr: &Transaction,
    prev_id: Option<NodeId>,
    next_id: Option<NodeId>,
    parent_id: NodeId,
    removed_index: usize,
) -> Selection {
    let doc = tr.doc();

    if let Some(next_id) = next_id
        && let Some(next) = doc.node(next_id)
        && matches!(next.node(), Node::Text(_))
    {
        return Selection::collapsed(Position {
            node_id: next_id,
            offset: 0,
            affinity: Affinity::Downstream,
        });
    }

    if let Some(prev_id) = prev_id
        && let Some(prev) = doc.node(prev_id)
        && let Node::Text(t) = prev.node()
    {
        return Selection::collapsed(Position {
            node_id: prev_id,
            offset: t.text.len(),
            affinity: Affinity::Upstream,
        });
    }

    Selection::collapsed(Position {
        node_id: parent_id,
        offset: removed_index,
        affinity: Affinity::Downstream,
    })
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
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 3)
        };
        transact_fail!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
    }

    #[test]
    fn delete_in_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Helo") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_at_end_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hell") } } }
            selection: (t1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_at_start_of_text_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
    }

    #[test]
    fn delete_at_start_with_prev_text_sibling() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("World")
                    }
                }
            }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hell")
                        t2: text("World")
                    }
                }
            }
            selection: (t1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_single_char_removes_node() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("X")
                        t3: text("World")
                    }
                }
            }
            selection: (t2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t3: text("World")
                    }
                }
            }
            selection: (t3, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_prev_single_char_removes_prev_node() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("X")
                        t2: text("Hello")
                    }
                }
            }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t2: text("Hello")
                    }
                }
            }
            selection: (t2, 0)
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
            doc { root { paragraph { t1: text("한글") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("한") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_last_char_lifts_first_text_marker_to_paragraph() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { t1: text("A") [bold] } } }
            selection: (t1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let p = actual.doc.node(p1).unwrap();
        let marker = p.marker().expect("paragraph should have a marker");
        assert!(marker.modifiers.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn delete_non_last_char_no_lift() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hi") [bold] } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_backward(
            &mut tr,
            &Resource::new_test()
        ));
        let p = actual.doc.node(p1).unwrap();
        assert_eq!(p.modifiers().count(), 0);
    }
}
