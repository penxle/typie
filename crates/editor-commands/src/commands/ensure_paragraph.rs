use editor_model::{NodeId, NodeType, PlainNode, PlainParagraphNode, Subtree};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::helpers::is_block_container;
use crate::{CommandError, CommandResult};

pub fn ensure_paragraph(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.is_collapsed() {
        return Ok(false);
    }

    let doc = tr.doc();
    let resolved = selection
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
    let from = Position::from(resolved.from());
    let to = Position::from(resolved.to());

    if from.node_id != to.node_id {
        return Ok(false);
    }

    let parent = doc
        .node(from.node_id)
        .ok_or(CommandError::NodeNotFound(from.node_id))?;

    if !is_block_container(parent.node()) {
        return Ok(false);
    }

    if !parent.spec().content.matches(NodeType::Paragraph) {
        return Ok(false);
    }

    let parent_id = parent.id();
    let children_to_remove: Vec<NodeId> = parent
        .entry()
        .children
        .iter()
        .skip(from.offset)
        .take(to.offset - from.offset)
        .copied()
        .collect();

    let new_para_id = NodeId::new();

    tr.batch::<_, CommandError>(|tr| {
        for child_id in children_to_remove.into_iter().rev() {
            tr.remove_subtree(child_id)?;
        }

        let subtree = Subtree::leaf(
            new_para_id,
            PlainNode::Paragraph(PlainParagraphNode::default()),
        );
        tr.insert_subtree(parent_id, from.offset, subtree)?;

        let doc = tr.doc();
        if let Some(parent) = doc.node(parent_id) {
            tr.apply_steps(fulfill(&parent))?;
        }
        Ok(())
    })?;

    tr.set_selection(Some(Selection::collapsed(Position {
        node_id: new_para_id,
        offset: 0,
        affinity: Affinity::Downstream,
    })))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn replaces_single_leaf_with_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                p1: paragraph
                paragraph { text("c") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        transact_fail!(initial, |tr| ensure_paragraph(&mut tr));
    }

    #[test]
    fn range_within_text_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        transact_fail!(initial, |tr| ensure_paragraph(&mut tr));
    }

    #[test]
    fn cross_textblock_range_returns_false() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("a") }
                paragraph { t2: text("b") }
            } }
            selection: (t1, 0) -> (t2, 1)
        };
        transact_fail!(initial, |tr| ensure_paragraph(&mut tr));
    }

    #[test]
    fn range_with_text_endpoint_returns_false() {
        let (initial, ..) = state! {
            doc { r: root {
                horizontal_rule
                paragraph { t1: text("hello") }
            } }
            selection: (r, 0, >) -> (t1, 3)
        };
        transact_fail!(initial, |tr| ensure_paragraph(&mut tr));
    }

    #[test]
    fn replaces_multiple_leaves_with_single_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                horizontal_rule
                horizontal_rule
            } }
            selection: (r, 0, >) -> (r, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replaces_mixed_blocks_and_textblock_with_single_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                horizontal_rule
                paragraph { text("middle") }
                horizontal_rule
            } }
            selection: (r, 0, >) -> (r, 3, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replaces_single_textblock_selection() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("hello") }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replaces_all_children_then_fulfills() {
        let (initial, ..) = state! {
            doc { r: root {
                horizontal_rule
                horizontal_rule
                horizontal_rule
            } }
            selection: (r, 0, >) -> (r, 3, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replaces_selection_inside_blockquote() {
        let (initial, ..) = state! {
            doc { root {
                bq: blockquote {
                    paragraph { text("x") }
                    paragraph { text("y") }
                }
            } }
            selection: (bq, 0, >) -> (bq, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph
                    paragraph { text("y") }
                }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replaces_selection_inside_fold_content() {
        let (initial, ..) = state! {
            doc { root {
                fold {
                    fold_title { text("title") }
                    fc: fold_content {
                        paragraph { text("x") }
                        horizontal_rule
                        paragraph { text("y") }
                    }
                }
            } }
            selection: (fc, 1, >) -> (fc, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { text("title") }
                    fold_content {
                        paragraph { text("x") }
                        p1: paragraph
                        paragraph { text("y") }
                    }
                }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replaces_selection_inside_list_item() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list {
                    li: list_item {
                        paragraph { text("hello") }
                    }
                }
            } }
            selection: (li, 0, >) -> (li, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item {
                        p1: paragraph
                    }
                }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn parent_disallows_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc { root {
                bl: bullet_list {
                    list_item { paragraph { text("a") } }
                    list_item { paragraph { text("b") } }
                }
            } }
            selection: (bl, 0, >) -> (bl, 2, <)
        };
        transact_fail!(initial, |tr| ensure_paragraph(&mut tr));
    }
}
