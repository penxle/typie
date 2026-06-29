use editor_crdt::Dot;
use editor_model::{ChildView, Node, NodeType, PlainNode, PlainParagraphNode, Subtree};
use editor_transaction::Transaction;

use crate::helpers::place_caret_at_block_start;
use crate::{CommandError, CommandResult};

pub fn unwrap_fold(tr: &mut Transaction, node_id: Dot) -> CommandResult {
    let (parent_id, fold_index, has_title_text, title_text, content_blocks) = {
        let view = tr.view();
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
        let content_blocks: Vec<(Dot, NodeType)> = content
            .child_blocks()
            .map(|b| (b.id(), b.node_type()))
            .collect();

        let mut new_seq: Vec<NodeType> = parent.child_blocks().map(|b| b.node_type()).collect();
        new_seq.remove(fold_index);
        let mut insert_at = fold_index;
        if has_title_text {
            new_seq.insert(insert_at, NodeType::Paragraph);
            insert_at += 1;
        }
        for (i, (_, t)) in content_blocks.iter().enumerate() {
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
            content_blocks,
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
                    .and_then(|p| p.child_blocks().nth(next_index))
                    .map(|b| b.id())
                    .ok_or(CommandError::NodeNotFound(parent_id))?
            };
            if !title_text.is_empty() {
                tr.insert_text(new_para, 0, &title_text)?;
            }
            new_para_id = Some(new_para);
            next_index += 1;
        }
        for (i, (child_id, _)) in content_blocks.iter().enumerate() {
            tr.move_node(*child_id, parent_id, next_index + i)?;
        }
        tr.remove_subtree(node_id)?;
        Ok(())
    })?;

    let first_lifted_id = match new_para_id {
        Some(id) => id,
        None => {
            // move_node re-mints the moved block, so the captured content id is
            // stale; re-resolve the first lifted block at the fold's old slot.
            let view = tr.view();
            view.node(parent_id)
                .and_then(|p| p.child_blocks().nth(fold_index))
                .map(|b| b.id())
                .ok_or_else(|| CommandError::Corrupted("lifted content missing".into()))?
        }
    };
    place_caret_at_block_start(tr, first_lifted_id)?;
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
