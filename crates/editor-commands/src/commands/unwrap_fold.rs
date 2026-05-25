use editor_model::{Node, NodeId, NodeType, PlainNode, PlainParagraphNode, Subtree};
use editor_transaction::Transaction;

use crate::helpers::place_caret_at_block_start;
use crate::{CommandError, CommandResult};

pub fn unwrap_fold(tr: &mut Transaction, node_id: NodeId) -> CommandResult {
    let doc = tr.doc();
    let fold = doc
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
        .ok_or(CommandError::Corrupted("fold has no index".into()))?;

    let title = fold
        .first_child()
        .ok_or(CommandError::Corrupted("fold missing FoldTitle".into()))?;
    let content = fold
        .last_child()
        .ok_or(CommandError::Corrupted("fold missing FoldContent".into()))?;

    let title_text_ids: Vec<NodeId> = title.children().map(|c| c.id()).collect();
    let content_block_ids: Vec<(NodeId, NodeType)> =
        content.children().map(|c| (c.id(), c.as_type())).collect();

    let has_title_text = !title_text_ids.is_empty();

    let mut new_seq: Vec<NodeType> = parent.children().map(|c| c.as_type()).collect();
    new_seq.remove(fold_index);
    let mut insert_at = fold_index;
    if has_title_text {
        new_seq.insert(insert_at, NodeType::Paragraph);
        insert_at += 1;
    }
    for (i, (_, t)) in content_block_ids.iter().enumerate() {
        new_seq.insert(insert_at + i, *t);
    }
    if !parent_spec.content.matches_sequence(&new_seq) {
        return Ok(false);
    }

    let new_para_id = if has_title_text {
        Some(NodeId::new())
    } else {
        None
    };

    tr.batch::<_, CommandError>(|tr| {
        let mut next_index = fold_index + 1;
        if let Some(para_id) = new_para_id {
            tr.insert_subtree(
                parent_id,
                next_index,
                Subtree::leaf(para_id, PlainNode::Paragraph(PlainParagraphNode::default())),
            )?;
            next_index += 1;
            for (i, text_id) in title_text_ids.iter().enumerate() {
                tr.move_node(*text_id, para_id, i)?;
            }
        }
        for (i, (child_id, _)) in content_block_ids.iter().enumerate() {
            tr.move_node(*child_id, parent_id, next_index + i)?;
        }
        tr.remove_subtree(node_id)?;
        Ok(())
    })?;

    let first_lifted_id = new_para_id.unwrap_or(content_block_ids[0].0);
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
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { tt: text("title") }
                    paragraph { text("body") }
                    paragraph {}
                }
            }
            selection: (tt, 0)
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
                            paragraph { tb: text("body") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (f, 0)
        };
        let (actual, ..) = transact!(initial, |tr| unwrap_fold(&mut tr, f));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { tb: text("body") }
                    paragraph {}
                }
            }
            selection: (tb, 0)
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
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { tt: text("t") }
                    paragraph { text("a") }
                    bullet_list {
                        list_item { paragraph { text("b") } }
                    }
                    paragraph { text("c") }
                    paragraph {}
                }
            }
            selection: (tt, 0)
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
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("outer") }
                        fold_content {
                            paragraph { tt: text("inner") }
                            paragraph { text("body") }
                        }
                    }
                }
            }
            selection: (tt, 0)
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
