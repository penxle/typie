use editor_model::{ChildView, NodeType, PlainNode, PlainParagraphNode, Subtree};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{is_block_container, remove_child_at};
use crate::{CommandError, CommandResult};

pub fn ensure_paragraph(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor == selection.head {
        return Ok(false);
    }

    let (parent_id, from_offset, remove_count) = {
        let view = tr.state().view();
        let resolved = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let from = resolved.from();
        let to = resolved.to();

        let from_node = from.node();
        if from_node != to.node() {
            return Ok(false);
        }

        let parent = view
            .node(from_node)
            .ok_or(CommandError::NodeNotFound(from_node))?;

        if !is_block_container(&parent) {
            return Ok(false);
        }

        if !parent.spec().content.matches(NodeType::Paragraph) {
            return Ok(false);
        }

        let from_offset = from.offset();
        let to_offset = to.offset();
        let remove_count = to_offset - from_offset;

        (from_node, from_offset, remove_count)
    };

    tr.batch::<_, CommandError>(|tr| {
        for index in (from_offset..from_offset + remove_count).rev() {
            remove_child_at(tr, parent_id, index)?;
        }

        let subtree = Subtree::leaf(PlainNode::Paragraph(PlainParagraphNode::default()));
        tr.insert_subtree(parent_id, from_offset, subtree)?;

        let steps = {
            let view = tr.state().view();
            let parent = view
                .node(parent_id)
                .ok_or(CommandError::NodeNotFound(parent_id))?;
            fulfill(&parent)
        };
        tr.apply_steps(steps)?;
        Ok(())
    })?;

    let new_para_id = {
        let view = tr.state().view();
        let parent = view
            .node(parent_id)
            .ok_or(CommandError::NodeNotFound(parent_id))?;
        match parent.child_at(from_offset) {
            Some(ChildView::Block(b)) => b.id(),
            _ => return Err(CommandError::Corrupted("inserted paragraph missing".into())),
        }
    };

    tr.set_selection(Some(Selection::collapsed(Position {
        node: new_para_id,
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
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        transact_fail!(initial, |tr| ensure_paragraph(&mut tr));
    }

    #[test]
    fn range_within_text_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        transact_fail!(initial, |tr| ensure_paragraph(&mut tr));
    }

    #[test]
    fn cross_textblock_range_returns_false() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("a") }
                p2: paragraph { text("b") }
            } }
            selection: (p1, 0) -> (p2, 1)
        };
        transact_fail!(initial, |tr| ensure_paragraph(&mut tr));
    }

    #[test]
    fn range_with_text_endpoint_returns_false() {
        let (initial, ..) = state! {
            doc { r: root {
                horizontal_rule
                p1: paragraph { text("hello") }
            } }
            selection: (r, 0, >) -> (p1, 3)
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
