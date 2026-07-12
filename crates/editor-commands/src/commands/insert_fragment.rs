use editor_model::{ChildView, Fragment};
use editor_state::first_cursor_position;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{find_ancestor_textblock, materialize_caret_block};
use crate::{CommandError, CommandResult};

pub fn insert_fragment(tr: &mut Transaction, fragment: Fragment) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let node_type = fragment.node.as_type();
    {
        let view = tr.view();
        let Some(textblock_id) = find_ancestor_textblock(&view, selection.head.node) else {
            return Ok(false);
        };
        let textblock = view
            .node(textblock_id)
            .ok_or(CommandError::NodeNotFound(textblock_id))?;
        let parent = textblock
            .parent()
            .ok_or(CommandError::NoParent(textblock_id))?;
        if !parent.spec().content.matches(node_type) {
            return Ok(false);
        }
    }

    materialize_caret_block(tr)?;

    let selection = tr
        .selection()
        .ok_or_else(|| CommandError::Corrupted("materialized caret selection missing".into()))?;
    let pos = selection.head;

    let (parent_id, textblock_id, textblock_index, child_count, is_empty) = {
        let view = tr.view();
        let Some(textblock_id) = find_ancestor_textblock(&view, pos.node) else {
            return Ok(false);
        };
        let textblock = view
            .node(textblock_id)
            .ok_or(CommandError::NodeNotFound(textblock_id))?;
        let parent = textblock
            .parent()
            .ok_or(CommandError::NoParent(textblock_id))?;

        let parent_id = parent.id();
        let textblock_index = textblock
            .index()
            .ok_or(CommandError::orphan_child(textblock_id, parent_id))?;
        let child_count = textblock.children().count();
        (
            parent_id,
            textblock_id,
            textblock_index,
            child_count,
            child_count == 0,
        )
    };

    let subtree = fragment.into_subtree();

    let insert_index = if is_empty || pos.offset == 0 {
        textblock_index
    } else {
        textblock_index + 1
    };

    tr.batch::<_, CommandError>(|tr| {
        if is_empty {
            tr.remove_subtree(textblock_id)?;
            tr.insert_subtree(parent_id, textblock_index, subtree)?;
        } else if pos.offset == 0 {
            tr.insert_subtree(parent_id, textblock_index, subtree)?;
        } else if pos.offset >= child_count {
            tr.insert_subtree(parent_id, textblock_index + 1, subtree)?;
        } else {
            tr.split_node(textblock_id, pos.offset)?;
            tr.insert_subtree(parent_id, textblock_index + 1, subtree)?;
        }
        Ok(())
    })?;

    let steps = {
        let view = tr.view();
        let mut steps = Vec::new();
        if let Some(ChildView::Block(b)) =
            view.node(parent_id).and_then(|p| p.child_at(insert_index))
        {
            steps.extend(fulfill(&b));
        }
        if let Some(parent) = view.node(parent_id) {
            steps.extend(fulfill(&parent));
        }
        steps
    };
    tr.apply_steps(steps)?;

    let bracket = Selection::new(
        Position {
            node: parent_id,
            offset: insert_index,
            affinity: Affinity::Downstream,
        },
        Position {
            node: parent_id,
            offset: insert_index + 1,
            affinity: Affinity::Upstream,
        },
    );

    let selection = {
        let view = tr.view();
        let parent = view
            .node(parent_id)
            .ok_or(CommandError::NodeNotFound(parent_id))?;
        match parent.child_at(insert_index) {
            // A real container block (e.g. a fulfilled Blockquote) gets the caret
            // placed at its first inner cursor position; only leaf/atom units are
            // bracketed as a whole.
            Some(ChildView::Block(b)) => match first_cursor_position(&b) {
                Some(pos) => Some(Selection::collapsed(pos)),
                None => Some(bracket),
            },
            Some(ChildView::Leaf(_)) => Some(bracket),
            None => None,
        }
    };

    if let Some(selection) = selection {
        tr.set_selection(Some(selection))?;
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
    fn insert_fragment_returns_false_when_no_selection() {
        let (initial, ..) = state! {
            doc { root { paragraph { text("Hello") } } }
            selection: none
        };
        let mut tr = Transaction::new(&initial);
        let result = insert_fragment(&mut tr, hr_fragment());
        assert!(matches!(result, Ok(false)));
    }

    #[test]
    fn rejects_range_selection() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 3)
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
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
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
                p1: paragraph { text("Hello") }
                paragraph {}
            } }
            selection: (p1, 5)
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
            doc { root { p1: paragraph { text("HelloWorld") } } }
            selection: (p1, 5)
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
            doc { root { p1: paragraph { text("Hello") hard_break text("World") } } }
            selection: (p1, 6)
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
    fn split_by_fragment_leaves_right_carry_empty_and_left_unchanged() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph carry([bold]) { text("HelloWorld") } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));
        let view = actual.view();
        let blocks: Vec<_> = view.root().unwrap().child_blocks().collect();
        let left = blocks[0].id();
        let right = blocks[1].id();
        assert_eq!(
            left, p1,
            "the original paragraph is the left half of the split"
        );
        let left_carry = actual.projected.carry_modifiers(left);
        assert!(
            left_carry.values().any(|m| matches!(m, Modifier::Bold)),
            "the original left paragraph keeps its carry, got {left_carry:?}"
        );
        assert!(
            actual.projected.carry_modifiers(right).is_empty(),
            "the split-off right paragraph has no carry (no implicit copy on a non-command split), got {:?}",
            actual.projected.carry_modifiers(right)
        );
    }

    #[test]
    fn rejects_when_parent_disallows_node_type() {
        let (initial, ..) = state! {
            doc { root { blockquote { p1: paragraph { text("Hello") } } } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));
    }

    #[test]
    fn rejects_disallowed_fragment_without_materializing_synthetic_caret() {
        let (mut initial, ..) = state! {
            doc { root { blockquote paragraph {} } }
            selection: none
        };
        let body = {
            let view = initial.view();
            view.root()
                .unwrap()
                .child_blocks()
                .find(|block| block.node_type() == NodeType::Blockquote)
                .unwrap()
                .child_blocks()
                .next()
                .unwrap()
                .id()
        };
        assert!(
            body.as_op_dot().is_none(),
            "fixture caret must be synthetic"
        );
        initial.selection = Some(Selection::collapsed(Position::new(body, 0)));
        let expected = initial.clone();

        let (actual, ..) = transact_fail!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));

        assert_state_eq!(&actual, &expected);
        assert_eq!(actual.selection.unwrap().head.node, body);
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
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
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

    #[test]
    fn materializes_synthetic_fold_content_before_inserting_fragment() {
        let (mut initial, ..) = state! {
            doc { root { fold paragraph {} } }
            selection: none
        };
        let body = {
            let view = initial.view();
            let fold = view
                .root()
                .unwrap()
                .child_blocks()
                .find(|block| block.node_type() == NodeType::Fold)
                .unwrap();
            let content = fold
                .child_blocks()
                .find(|block| block.node_type() == NodeType::FoldContent)
                .unwrap();
            content.child_blocks().next().unwrap().id()
        };
        initial.selection = Some(Selection::collapsed(Position::new(body, 0)));

        let (actual, ..) = transact!(initial, |tr| insert_fragment(&mut tr, hr_fragment()));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title {}
                    content: fold_content { horizontal_rule }
                }
                paragraph {}
            } }
            selection: (content, 0, >) -> (content, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }
}
