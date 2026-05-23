use editor_model::{Node, NodeId};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::find_enclosing_list_item_id;
use crate::{CommandError, CommandResult};

pub fn split_list_item(tr: &mut Transaction) -> CommandResult {
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

    let list_item_id = match find_enclosing_list_item_id(&doc, pos.node_id) {
        Some(id) => id,
        None => return Ok(false),
    };

    let list_item = doc
        .node(list_item_id)
        .ok_or(CommandError::NodeNotFound(list_item_id))?;
    let paragraph = list_item.first_child().ok_or(CommandError::Corrupted(
        "list_item missing paragraph".into(),
    ))?;
    let paragraph_id = paragraph.id();

    if paragraph.first_child().is_none() {
        return Ok(false);
    }

    // Decide whether a mid-text split is needed and where to split the paragraph.
    // The structural changes themselves run inside tr.batch so the whole sequence
    // is atomic against transaction failure.
    enum SplitPlan {
        Text {
            text_id: NodeId,
            text_offset: usize,
            paragraph_split_index: usize,
        },
        Direct {
            paragraph_split_index: usize,
        },
    }

    let split_plan = match node.node() {
        Node::Text(text_node) => {
            let parent = node.parent().ok_or(CommandError::NoParent(pos.node_id))?;
            if parent.id() != paragraph_id {
                return Ok(false);
            }
            let node_index = node
                .index()
                .ok_or(CommandError::orphan_child(pos.node_id, parent.id()))?;
            let text_len = text_node.text.len();
            if pos.offset == 0 {
                SplitPlan::Direct {
                    paragraph_split_index: node_index,
                }
            } else if pos.offset == text_len {
                SplitPlan::Direct {
                    paragraph_split_index: node_index + 1,
                }
            } else {
                SplitPlan::Text {
                    text_id: pos.node_id,
                    text_offset: pos.offset,
                    paragraph_split_index: node_index + 1,
                }
            }
        }
        Node::Paragraph(_) => {
            if node.id() != paragraph_id {
                return Ok(false);
            }
            SplitPlan::Direct {
                paragraph_split_index: pos.offset,
            }
        }
        _ => return Ok(false),
    };

    let new_paragraph_id = NodeId::new();
    let new_list_item_id = NodeId::new();

    tr.batch::<_, CommandError>(|tr| {
        let paragraph_split_index = match split_plan {
            SplitPlan::Text {
                text_id,
                text_offset,
                paragraph_split_index,
            } => {
                let split_text_id = NodeId::new();
                tr.split_node(text_id, text_offset, split_text_id)?;
                paragraph_split_index
            }
            SplitPlan::Direct {
                paragraph_split_index,
            } => paragraph_split_index,
        };
        tr.split_node(paragraph_id, paragraph_split_index, new_paragraph_id)?;
        tr.split_node(list_item_id, 1, new_list_item_id)?;
        Ok(())
    })?;

    let doc = tr.doc();
    let new_li = doc
        .node(new_list_item_id)
        .ok_or(CommandError::NodeNotFound(new_list_item_id))?;
    let new_para = new_li.first_child().ok_or(CommandError::Corrupted(
        "new list_item missing paragraph".into(),
    ))?;
    let cursor_pos = match new_para.first_child() {
        Some(child) if matches!(child.node(), Node::Text(_)) => Position {
            node_id: child.id(),
            offset: 0,
            affinity: Affinity::Downstream,
        },
        _ => Position {
            node_id: new_para.id(),
            offset: 0,
            affinity: Affinity::Downstream,
        },
    };
    tr.set_selection(Some(Selection::collapsed(cursor_pos)))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn split_text_end() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("Hello") } }
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
        let (initial, ..) = state! {
            doc { root { bullet_list { list_item { paragraph { t1: text("A") } } } paragraph {} } }
            selection: (t1, 0) -> (t1, 1)
        };
        transact_fail!(initial, |tr| split_list_item(&mut tr));
    }

    #[test]
    fn empty_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc { root { bullet_list { list_item { p1: paragraph {} } } paragraph {} } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| split_list_item(&mut tr));
    }

    #[test]
    fn outside_list_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        transact_fail!(initial, |tr| split_list_item(&mut tr));
    }

    #[test]
    fn split_text_middle() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("He") } }
                        list_item { paragraph { t2: text("llo") } }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_text_start() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph {} }
                        list_item { paragraph { t1: text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_with_sublist_moves_sublist_to_new_item() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { t1: text("Hello") }
                            bullet_list { list_item { paragraph { text("sub") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("Hello") } }
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
    fn split_in_ordered_list() {
        let (initial, ..) = state! {
            doc {
                root {
                    ordered_list {
                        list_item { paragraph { t1: text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    ordered_list {
                        list_item { paragraph { t1: text("He") } }
                        list_item { paragraph { t2: text("llo") } }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_multiple_text_children() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph {
                                t1: text("Hello")
                                t2: text("World")
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("Hello") } }
                        list_item { paragraph { t2: text("World") } }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
