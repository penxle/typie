use editor_common::StrExt;
use editor_model::{Node, NodeId};
use editor_resource::Resource;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn delete_text_forward(tr: &mut Transaction, resource: &Resource) -> CommandResult {
    let selection = tr.selection();
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

    let text_len = text_node.text.char_count();

    if pos.offset < text_len {
        let doc = tr.doc();
        let next_offset = pos
            .resolve(&doc)
            .and_then(|r| r.next_grapheme(resource))
            .map(|r| r.offset())
            .unwrap_or(pos.offset.saturating_add(1).min(text_len));
        let delete_count = next_offset - pos.offset;
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
            tr.set_selection(new_selection)?;
        } else {
            tr.remove_text(pos.node_id, pos.offset, delete_count)?;
            tr.set_selection(Selection::collapsed(Position {
                node_id: pos.node_id,
                offset: pos.offset,
                affinity: Affinity::Downstream,
            }))?;
        }
    } else {
        // offset == len: try deleting first char of next text sibling
        let next = match node.next_sibling() {
            Some(next) => next,
            None => return Ok(false),
        };

        let Node::Text(next_text) = next.node() else {
            return Ok(false);
        };

        let next_id = next.id();
        let is_last_char = next_text.text.char_count() == 1;

        if is_last_char {
            tr.remove_subtree(next_id)?;
            tr.set_selection(Selection::collapsed(Position {
                node_id: pos.node_id,
                offset: pos.offset,
                affinity: Affinity::Upstream,
            }))?;
        } else {
            tr.remove_text(next_id, 0, 1)?;
            tr.set_selection(Selection::collapsed(Position {
                node_id: pos.node_id,
                offset: pos.offset,
                affinity: Affinity::Upstream,
            }))?;
        }
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
            offset: t.text.char_count(),
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
    use editor_resource::Resource;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 3)
        };
        transact_fail!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
    }

    #[test]
    fn delete_at_start_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ello") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_in_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_forward(
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
    fn delete_at_end_of_text_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        transact_fail!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
    }

    #[test]
    fn delete_at_end_with_next_text_sibling() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("World")
                    }
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("orld")
                    }
                }
            }
            selection: (t1, 5)
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
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_forward(
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
    fn delete_next_single_char_removes_next_node() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("X")
                    }
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 5)
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
            doc { root { paragraph { t1: text("한글") } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_text_forward(
            &mut tr,
            &Resource::new_test()
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("글") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
