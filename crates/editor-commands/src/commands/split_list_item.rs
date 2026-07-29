use editor_model::{ChildView, NodeType, Subtree};
use editor_state::{Position, Selection};
use editor_transaction::{StepError, Transaction};

use crate::helpers::{continuation_paint_at, find_enclosing_list_item_id};
use crate::{CommandError, CommandResult};

pub fn split_list_item(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;
    let view = tr.view();

    let list_item_id = match find_enclosing_list_item_id(&view, pos.node) {
        Some(id) => id,
        None => return Ok(false),
    };

    let list_item = view
        .node(list_item_id)
        .ok_or(CommandError::NodeNotFound(list_item_id))?;
    let paragraph = view
        .node(pos.node)
        .ok_or(CommandError::NodeNotFound(pos.node))?;
    if paragraph.node_type() != NodeType::Paragraph
        || paragraph.parent().map(|parent| parent.id()) != Some(list_item_id)
    {
        return Ok(false);
    }
    if list_item
        .children()
        .any(|child| matches!(child, ChildView::Leaf(_)))
    {
        return Err(CommandError::Corrupted(
            "list item contains an unsupported direct child".into(),
        ));
    }
    let paragraph_id = paragraph.id();
    let paragraph_index = paragraph
        .parent()
        .and_then(|item| {
            item.child_blocks()
                .position(|child| child.id() == paragraph_id)
        })
        .ok_or_else(|| CommandError::orphan_child(paragraph_id, list_item_id))?;

    let (list_id, li_block_index) = {
        let list = list_item
            .parent()
            .ok_or(CommandError::NoParent(list_item_id))?;
        let idx = list
            .child_blocks()
            .position(|b| b.id() == list_item_id)
            .ok_or_else(|| CommandError::orphan_child(list_item_id, list.id()))?;
        (list.id(), idx)
    };

    let paint = continuation_paint_at(&tr.state().projected, pos);
    drop(view);

    tr.batch::<_, CommandError>(|tr| {
        tr.split_node(paragraph_id, pos.offset)?;

        let moving = {
            let view = tr.view_clean().map_err(StepError::from)?;
            let item = view
                .node(list_item_id)
                .ok_or(CommandError::NodeNotFound(list_item_id))?;
            item.child_blocks()
                .skip(paragraph_index + 1)
                .map(|child| child.id())
                .collect::<Vec<_>>()
        };
        if moving.is_empty() {
            return Err(CommandError::Corrupted(
                "list item split produced no right paragraph".into(),
            ));
        }

        let (_, moved) = tr.insert_subtree_with_moved(
            list_id,
            li_block_index + 1,
            Subtree::leaf(NodeType::ListItem.into_node().to_plain()),
            &moving,
        )?;
        let new_paragraph_id =
            moved
                .first()
                .map(|moved| moved.root)
                .ok_or(CommandError::Corrupted(
                    "list item split moved no paragraph".into(),
                ))?;

        tr.replace_carry(paragraph_id, paint.clone())?;
        tr.replace_carry(new_paragraph_id, paint.clone())?;
        tr.set_selection(Some(Selection::collapsed(Position::new(
            new_paragraph_id,
            0,
        ))))?;
        Ok(())
    })?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn split_text_end() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                        list_item { p2: paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn non_collapsed_returns_false() {
        let (initial, _p1) = state! {
            doc { root { bullet_list { list_item { p1: paragraph { text("A") } } } paragraph {} } }
            selection: (p1, 0) -> (p1, 1)
        };
        transact_fail!(initial, |tr| split_list_item(&mut tr));
    }

    #[test]
    fn split_empty_paragraph_at_command_layer() {
        let (initial, ..) = state! {
            doc { root { bullet_list { list_item { p1: paragraph {} } } paragraph {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph {} }
                        list_item { p2: paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    /// `split_node` cannot split through an unknown-bearing inline leaf
    /// losslessly. The surrounding batch must therefore roll back the attempted
    /// split rather than leaving a partial item partition.
    #[test]
    fn split_rejects_unknown_leaf_in_tail_leaving_doc_unchanged() {
        use editor_crdt::ListOp;
        use editor_model::{ChildView, EditOp, SeqItem};

        let (base, p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("ab") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let a_dot = {
            let view = base.view();
            match view.node(p1).unwrap().child_at(0).unwrap() {
                ChildView::Leaf(l) => l.dot(),
                ChildView::Block(_) => panic!("expected char leaf"),
            }
        };
        let pos = base.projected.seq_flat_pos(a_dot).unwrap() + 1;
        let mut initial = base;
        initial
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos,
                item: SeqItem::Unknown {
                    tag: 1,
                    bytes: vec![],
                },
            }))
            .unwrap();
        let before = initial.clone();

        let mut tr = editor_transaction::Transaction::new(&initial);
        let result = split_list_item(&mut tr);
        assert!(
            result.is_err(),
            "split must reject a tail containing an unsupported (unknown) node"
        );
        let (after, records, ..) = tr.commit();
        assert!(
            records.is_empty(),
            "a rejected split must not have applied any steps"
        );
        assert_state_eq!(&after, &before);
    }

    #[test]
    fn outside_list_returns_false() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        transact_fail!(initial, |tr| split_list_item(&mut tr));
    }

    #[test]
    fn split_text_middle() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("He") } }
                        list_item { p2: paragraph { text("llo") } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_before_tab_preserves_tail_inline_atom() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("a") tab text("b") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("a") } }
                        list_item { p2: paragraph { tab text("b") } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_before_tab_preserves_tail_atom_modifiers() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("a") tab [font_size(2400)] text("b") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("a") } }
                        list_item { p2: paragraph { tab [font_size(2400)] text("b") } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_text_start() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph {} }
                        list_item { p1: paragraph { text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_list_item_at_start_carries_right_paint_to_both() {
        use editor_model::ModifierType;
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") [bold] } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (first, second) = {
            let view = actual.view();
            let list = view
                .root()
                .unwrap()
                .child_blocks()
                .find(|b| b.node_type() == editor_model::NodeType::BulletList)
                .unwrap();
            let mut items = list.child_blocks();
            let a = items.next().unwrap();
            let b = items.next().unwrap();
            let para_of = |li: &editor_model::NodeView| match li.first_child() {
                Some(editor_model::ChildView::Block(p)) => p.id(),
                _ => unreachable!(),
            };
            (para_of(&a), para_of(&b))
        };
        for para in [first, second] {
            assert!(
                actual
                    .projected
                    .carry_modifiers(para)
                    .contains_key(&ModifierType::Bold),
                "splitting at the run start carries the right neighbor's paint into both list items"
            );
        }
    }

    #[test]
    fn split_with_sublist_moves_sublist_to_new_item() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("Hello") }
                            bullet_list { list_item { paragraph { text("sub") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                        list_item {
                            p2: paragraph {}
                            bullet_list { list_item { paragraph { text("sub") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_middle_direct_paragraph_moves_following_siblings() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("before") }
                            bullet_list {
                                list_item { paragraph { text("nested-before") } }
                            }
                            p1: paragraph { text("hello") }
                            ordered_list {
                                list_item { paragraph { text("nested-after") } }
                            }
                            paragraph { text("after") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("before") }
                            bullet_list {
                                list_item { paragraph { text("nested-before") } }
                            }
                            paragraph { text("he") }
                        }
                        list_item {
                            p2: paragraph { text("llo") }
                            ordered_list {
                                list_item { paragraph { text("nested-after") } }
                            }
                            paragraph { text("after") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }

    #[test]
    fn split_moves_sublist_preserving_inner_bold() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("Hello") }
                            bullet_list { list_item { paragraph { text("sub") [bold] } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                        list_item {
                            p2: paragraph {}
                            bullet_list { list_item { paragraph { text("sub") [bold] } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_in_ordered_list() {
        let (initial, _p1) = state! {
            doc {
                root {
                    ordered_list {
                        list_item { p1: paragraph { text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    ordered_list {
                        list_item { p1: paragraph { text("He") } }
                        list_item { p2: paragraph { text("llo") } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_multiple_text_children() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph {
                                text("Hello")
                                text("World")
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                        list_item { p2: paragraph { text("World") } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_list_item_at_end_attaches_marker_to_new_paragraph() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") [bold] } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph carry([bold]) { text("Hello") [bold] } }
                        list_item { p2: paragraph carry([bold]) {} }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_list_item_replaces_carry_on_both() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") [bold] } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph carry([bold]) { text("Hello") [bold] } }
                        list_item { p2: paragraph carry([bold]) {} }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_list_item_in_middle_attaches_marker_to_new_paragraph() {
        let (initial, _p1) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") [bold] } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, _p1, _p2) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph carry([bold]) { text("He") [bold] } }
                        list_item { p2: paragraph carry([bold]) { text("llo") [bold] } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
