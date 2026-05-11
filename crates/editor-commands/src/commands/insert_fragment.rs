use editor_model::{Fragment, Node, NodeId};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{find_ancestor_textblock, find_first_cursor_position};
use crate::{CommandError, CommandResult};

pub fn insert_fragment(tr: &mut Transaction, fragment: Fragment) -> CommandResult {
    let selection = tr.selection();
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();

    let Some(textblock_id) = find_ancestor_textblock(&doc, pos.node_id) else {
        return Ok(false);
    };

    let textblock = doc
        .node(textblock_id)
        .ok_or(CommandError::NodeNotFound(textblock_id))?;
    let parent = textblock
        .parent()
        .ok_or(CommandError::NoParent(textblock_id))?;

    let node_type = fragment.node.as_type();
    if !parent.spec().content.matches(node_type) {
        return Ok(false);
    }

    let parent_id = parent.id();
    let textblock_index = textblock
        .index()
        .ok_or(CommandError::orphan_child(textblock_id, parent_id))?;

    let subtree = fragment.into_subtree();
    let subtree_id = subtree.id;
    let is_empty = textblock.first_child().is_none();

    tr.batch::<_, CommandError>(|tr| {
        if is_empty {
            tr.remove_subtree(textblock_id)?;
            tr.insert_subtree(parent_id, textblock_index, subtree)?;
        } else {
            let node = doc
                .node(pos.node_id)
                .ok_or(CommandError::NodeNotFound(pos.node_id))?;

            match node.node() {
                Node::Text(text_node) => {
                    let node_index = node
                        .index()
                        .ok_or(CommandError::orphan_child(pos.node_id, textblock_id))?;
                    let text_len = text_node.text.len();

                    if pos.offset == 0 && node_index == 0 {
                        tr.insert_subtree(parent_id, textblock_index, subtree)?;
                    } else if pos.offset == text_len && node.next_sibling().is_none() {
                        tr.insert_subtree(parent_id, textblock_index + 1, subtree)?;
                    } else {
                        let split_index = if pos.offset == 0 {
                            node_index
                        } else if pos.offset == text_len {
                            node_index + 1
                        } else {
                            let split_text_id = NodeId::new();
                            tr.split_node(pos.node_id, pos.offset, split_text_id)?;
                            node_index + 1
                        };

                        let split_para_id = NodeId::new();
                        tr.split_node(textblock_id, split_index, split_para_id)?;
                        tr.insert_subtree(parent_id, textblock_index + 1, subtree)?;
                    }
                }
                _ => {
                    let children_len = node.entry().children.len();
                    if pos.offset == 0 {
                        tr.insert_subtree(parent_id, textblock_index, subtree)?;
                    } else if pos.offset >= children_len {
                        tr.insert_subtree(parent_id, textblock_index + 1, subtree)?;
                    } else {
                        let split_para_id = NodeId::new();
                        tr.split_node(textblock_id, pos.offset, split_para_id)?;
                        tr.insert_subtree(parent_id, textblock_index + 1, subtree)?;
                    }
                }
            }
        }

        let doc = tr.doc();
        if let Some(inserted) = doc.node(subtree_id) {
            tr.apply_steps(fulfill(&inserted))?;
        }
        if let Some(parent) = doc.node(parent_id) {
            tr.apply_steps(fulfill(&parent))?;
        }
        Ok(())
    })?;

    let doc = tr.doc();
    let inserted = doc
        .node(subtree_id)
        .ok_or(CommandError::NodeNotFound(subtree_id))?;

    if inserted.spec().is_leaf() {
        if let Some(next) = inserted.next_sibling()
            && let Some(pos) = find_first_cursor_position(&next)
        {
            tr.set_selection(Selection::collapsed(pos))?;
        } else {
            let parent = inserted
                .parent()
                .ok_or(CommandError::NoParent(subtree_id))?;
            let idx = inserted
                .index()
                .ok_or(CommandError::orphan_child(subtree_id, parent.id()))?;
            tr.set_selection(Selection::new(
                Position {
                    node_id: parent.id(),
                    offset: idx,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: parent.id(),
                    offset: idx + 1,
                    affinity: Affinity::Upstream,
                },
            ))?;
        }
    } else if let Some(pos) = find_first_cursor_position(&inserted) {
        tr.set_selection(Selection::collapsed(pos))?;
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;
    use editor_model::*;

    fn hr_fragment() -> Fragment {
        Fragment::leaf(PlainNode::HorizontalRule(PlainHorizontalRuleNode::default()))
    }

    #[test]
    fn rejects_range_selection() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 3)
        };
        transact_fail!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));
    }

    #[test]
    fn replaces_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));
        let (expected, ..) = state! {
            doc { r: root {
                horizontal_rule
                paragraph {}
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn inserts_before_textblock_at_start() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));
        let (expected, ..) = state! {
            doc { r: root {
                horizontal_rule
                paragraph { text("Hello") }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn inserts_after_textblock_at_end() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("Hello") }
                paragraph {}
            } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));
        let (expected, ..) = state! {
            doc { r: root {
                paragraph { text("Hello") }
                horizontal_rule
                paragraph {}
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn splits_paragraph_at_middle() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("HelloWorld") } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));
        let (expected, ..) = state! {
            doc {
                r: root {
                    paragraph { text("Hello") }
                    horizontal_rule
                    paragraph { text("World") }
                }
            }
            selection: (r, 1, >) -> (r, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn splits_paragraph_at_non_first_node_start() {
        let (initial, ..) = state! {
            doc { root { paragraph { text("Hello") hard_break t2: text("World") } } }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));
        let (expected, ..) = state! {
            doc {
                r: root {
                    paragraph { text("Hello") hard_break }
                    horizontal_rule
                    paragraph { text("World") }
                }
            }
            selection: (r, 1, >) -> (r, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn rejects_when_parent_disallows_node_type() {
        let (initial, ..) = state! {
            doc { root { blockquote { paragraph { t1: text("Hello") } } } }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));
    }

    #[test]
    fn inserts_in_root_direct_child_paragraph() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph {}
                paragraph {}
            } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));
        let (expected, ..) = state! {
            doc { r: root {
                horizontal_rule
                paragraph {}
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn inserts_at_end_creates_trailing_paragraph() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));
        let (expected, ..) = state! {
            doc { r: root {
                paragraph { text("Hello") }
                horizontal_rule
                paragraph {}
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn splits_at_container_position_middle() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { hard_break hard_break }
                paragraph {}
            } }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));
        let (expected, ..) = state! {
            doc {
                r: root {
                    paragraph { hard_break }
                    horizontal_rule
                    paragraph { hard_break }
                    paragraph {}
                }
            }
            selection: (r, 1, >) -> (r, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn fulfills_inserted_subtree() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_fragment(
            &mut tr,
            Fragment::leaf(PlainNode::Blockquote(PlainBlockquoteNode::default()))
        ));
        let (expected, ..) = state! {
            doc { root {
                blockquote { p: paragraph {} }
                paragraph {}
            } }
            selection: (p, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
