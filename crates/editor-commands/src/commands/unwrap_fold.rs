use editor_crdt::Dot;
use editor_model::{ChildView, Modifier, Node, NodeType, PlainNode, PlainParagraphNode, Subtree};
use editor_transaction::Transaction;

use crate::helpers::{capture_atom_leaf_subtree_at, child_node_type, place_caret_at_block_start};
use crate::{CommandError, CommandResult};

enum ContentChild {
    Block(Dot),
    Leaf(Subtree),
}

pub fn unwrap_fold(tr: &mut Transaction, node_id: Dot) -> CommandResult {
    let (parent_id, fold_index, has_title_text, title_text, title_carry, content_children) = {
        let state = tr.state();
        let view = state.view();
        let fold = view
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;
        if !matches!(fold.node(), Node::Fold(_)) {
            return Ok(false);
        }
        let parent = fold.parent().ok_or(CommandError::NoParent(node_id))?;
        let parent_id = parent.id();
        let parent_spec = parent.spec();
        let fold_index = fold
            .index()
            .ok_or_else(|| CommandError::Corrupted("fold has no index".into()))?;

        let title = match fold.first_child() {
            Some(ChildView::Block(t)) => t,
            _ => {
                return Err(CommandError::Corrupted("fold missing FoldTitle".into()));
            }
        };
        let content = match fold.last_child() {
            Some(ChildView::Block(c)) => c,
            _ => {
                return Err(CommandError::Corrupted("fold missing FoldContent".into()));
            }
        };

        let has_title_text = title.children().count() > 0;
        let title_text = title.inline_text();
        let title_carry: Vec<Modifier> = state
            .projected
            .carry_modifiers(title.id())
            .into_values()
            .collect();

        let mut content_children: Vec<(ContentChild, NodeType)> = Vec::new();
        for (i, c) in content.children().enumerate() {
            match c {
                ChildView::Block(b) => {
                    content_children.push((ContentChild::Block(b.id()), b.node_type()));
                }
                ChildView::Leaf(l) => {
                    let subtree = capture_atom_leaf_subtree_at(&state.projected, &content, i)?;
                    content_children.push((ContentChild::Leaf(subtree), l.node_type()));
                }
            }
        }

        let mut new_seq: Vec<NodeType> = parent.children().map(|c| child_node_type(&c)).collect();
        new_seq.remove(fold_index);
        let mut insert_at = fold_index;
        if has_title_text {
            new_seq.insert(insert_at, NodeType::Paragraph);
            insert_at += 1;
        }
        for (i, (_, t)) in content_children.iter().enumerate() {
            new_seq.insert(insert_at + i, *t);
        }
        if !parent_spec.content.matches_sequence(&new_seq) {
            return Ok(false);
        }

        (
            parent_id,
            fold_index,
            has_title_text,
            title_text,
            title_carry,
            content_children,
        )
    };

    let mut new_para_id: Option<Dot> = None;
    tr.batch::<_, CommandError>(|tr| {
        let mut next_index = fold_index + 1;
        if has_title_text {
            tr.insert_subtree(
                parent_id,
                next_index,
                Subtree::leaf(PlainNode::Paragraph(PlainParagraphNode::default())),
            )?;
            let new_para = {
                let view = tr.view();
                view.node(parent_id)
                    .and_then(|p| match p.child_at(next_index) {
                        Some(ChildView::Block(block)) => Some(block.id()),
                        _ => None,
                    })
                    .ok_or(CommandError::NodeNotFound(parent_id))?
            };
            if !title_text.is_empty() {
                tr.insert_text(new_para, 0, &title_text)?;
            }
            tr.replace_carry(new_para, title_carry.clone())?;
            new_para_id = Some(new_para);
            next_index += 1;
        }
        for (i, (child, _)) in content_children.iter().enumerate() {
            match child {
                ContentChild::Block(child_id) => {
                    tr.move_node(*child_id, parent_id, next_index + i)?;
                }
                ContentChild::Leaf(subtree) => {
                    tr.insert_subtree(parent_id, next_index + i, subtree.clone())?;
                }
            }
        }
        tr.remove_subtree(node_id)?;
        Ok(())
    })?;

    let caret_block = match new_para_id {
        Some(id) => Some(id),
        None => {
            // move_node re-mints the moved block, so the captured content id is
            // stale; re-resolve the first lifted block within the fold's old span.
            let view = tr.view();
            let content_len = content_children.len();
            view.node(parent_id).and_then(|p| {
                (fold_index..fold_index + content_len).find_map(|i| match p.child_at(i) {
                    Some(ChildView::Block(block)) => Some(block.id()),
                    _ => None,
                })
            })
        }
    };
    if let Some(block_id) = caret_block {
        place_caret_at_block_start(tr, block_id)?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn unwrap_fold_with_title_and_content() {
        let (initial, f, ..) = state! {
            doc {
                root {
                    f: fold {
                        fold_title { text("title") }
                        fold_content {
                            paragraph { text("body") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (f, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_fold(&mut tr, f));
        let (expected, _p1) = state! {
            doc {
                root {
                    p1: paragraph { text("title") }
                    paragraph { text("body") }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unwrap_fold_with_empty_title() {
        let (initial, f, ..) = state! {
            doc {
                root {
                    f: fold {
                        fold_title {}
                        fold_content {
                            p1: paragraph { text("body") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (f, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_fold(&mut tr, f));
        let (expected, _p1) = state! {
            doc {
                root {
                    p1: paragraph { text("body") }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unwrap_fold_after_image_keeps_content_in_fold_slot() {
        let (initial, f, ..) = state! {
            doc {
                root {
                    image
                    f: fold {
                        fold_title {}
                        fold_content {
                            p1: paragraph { text("body") }
                        }
                    }
                    paragraph { text("tail") }
                }
            }
            selection: (f, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_fold(&mut tr, f));
        let (expected, ..) = state! {
            doc {
                root {
                    image
                    p1: paragraph { text("body") }
                    paragraph { text("tail") }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unwrap_fold_with_multiple_content_blocks() {
        let (initial, f, ..) = state! {
            doc {
                root {
                    f: fold {
                        fold_title { text("t") }
                        fold_content {
                            paragraph { text("a") }
                            bullet_list {
                                list_item { paragraph { text("b") } }
                            }
                            paragraph { text("c") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (f, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_fold(&mut tr, f));
        let (expected, _p1) = state! {
            doc {
                root {
                    p1: paragraph { text("t") }
                    paragraph { text("a") }
                    bullet_list {
                        list_item { paragraph { text("b") } }
                    }
                    paragraph { text("c") }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unwrap_fold_nested_inside_fold_content() {
        let (initial, inner, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("outer") }
                        fold_content {
                            inner: fold {
                                fold_title { text("inner") }
                                fold_content {
                                    paragraph { text("body") }
                                }
                            }
                        }
                    }
                }
            }
            selection: (inner, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_fold(&mut tr, inner));
        let (expected, _p1) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("outer") }
                        fold_content {
                            p1: paragraph { text("inner") }
                            paragraph { text("body") }
                        }
                    }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unwrap_fold_transfers_title_carry_to_new_paragraph() {
        let (initial, f, ..) = state! {
            doc {
                root {
                    f: fold {
                        fold_title carry([bold]) { text("title") }
                        fold_content {
                            paragraph { text("body") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (f, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_fold(&mut tr, f));
        let new_para = {
            let view = actual.view();
            match view.root().unwrap().child_at(0) {
                Some(ChildView::Block(b)) => b.id(),
                _ => panic!("expected a lifted paragraph at root start"),
            }
        };
        assert_eq!(actual.view().node(new_para).unwrap().inline_text(), "title");
        let carry = actual.projected.carry_modifiers(new_para);
        assert!(
            carry
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Bold)),
            "unwrapping a fold whose title has text moves the title carry onto the new paragraph, got {carry:?}"
        );
    }

    #[test]
    fn unwrap_fold_empty_title_drops_carry() {
        let (initial, f, ..) = state! {
            doc {
                root {
                    f: fold {
                        fold_title carry([bold]) {}
                        fold_content {
                            p1: paragraph { text("body") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (f, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_fold(&mut tr, f));
        let (expected, _p1) = state! {
            doc {
                root {
                    p1: paragraph { text("body") }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unwrap_fold_preserves_content_leaf_atoms() {
        let (initial, f, ..) = state! {
            doc {
                root {
                    f: fold {
                        fold_title {}
                        fold_content {
                            p1: paragraph { text("a") }
                            image
                            paragraph { text("b") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (f, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_fold(&mut tr, f));
        let (expected, _p1) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    image
                    paragraph { text("b") }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unwrap_fold_preserves_content_image_block_modifier() {
        use editor_model::{Alignment, Modifier, ModifierType};
        use editor_transaction::Step;

        let (base, f) = state! {
            doc {
                root {
                    f: fold {
                        fold_title {}
                        fold_content {
                            image
                            paragraph { text("body") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (f, 0)
        };
        let img = {
            let view = base.view();
            let fold = view.node(f).unwrap();
            let content = match fold.last_child() {
                Some(ChildView::Block(c)) => c,
                _ => panic!("fold missing content"),
            };
            match content.child_at(0) {
                Some(ChildView::Leaf(l)) => l.dot(),
                _ => panic!("expected image leaf in fold content"),
            }
        };
        let center = Modifier::Alignment {
            value: Alignment::Center,
        };
        let mut prep = Transaction::new(&base);
        prep.apply_steps(vec![Step::AddModifier {
            block: img,
            modifier: center.clone(),
        }])
        .unwrap();
        let (initial, ..) = prep.commit();

        let (actual, ..) = transact!(initial, |tr| unwrap_fold(&mut tr, f));

        let view = actual.view();
        let root = view.root().unwrap();
        let new_img = match root.child_at(0) {
            Some(ChildView::Leaf(l)) => l.dot(),
            _ => panic!("expected the image leaf lifted to root start"),
        };
        assert_eq!(
            actual
                .projected
                .block_modifiers()
                .modifiers_of(new_img)
                .get(&ModifierType::Alignment),
            Some(&center),
            "unwrapping a fold preserves the content image's block alignment"
        );
        assert_eq!(
            match root.child_at(1) {
                Some(ChildView::Block(b)) => b.inline_text(),
                _ => panic!("expected the body paragraph after the image"),
            },
            "body"
        );
    }

    #[test]
    fn unwrap_non_fold_returns_false() {
        let (initial, bq, ..) = state! {
            doc {
                root {
                    bq: blockquote { paragraph { text("hi") } }
                    paragraph {}
                }
            }
            selection: (bq, 0)
        };
        transact_fail!(initial, |tr| unwrap_fold(&mut tr, bq));
    }
}
