use editor_model::{Node, NodeId, NodeType};
use editor_schema::NodeSpecExt;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, dissolve, prune};

use crate::{CommandError, CommandResult};

enum LiftDirection {
    Front,
    End,
}

pub fn lift_paragraph(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let paragraph_id = match node.node() {
        Node::Text(_) => {
            if pos.offset > 0 || node.prev_sibling().is_some() {
                return Ok(false);
            }
            node.parent()
                .ok_or(CommandError::NoParent(pos.node_id))?
                .id()
        }
        Node::Paragraph(_) => {
            if pos.offset > 0 {
                return Ok(false);
            }
            pos.node_id
        }
        _ => return Ok(false),
    };

    let doc = tr.doc();
    let paragraph = doc
        .node(paragraph_id)
        .ok_or(CommandError::NodeNotFound(paragraph_id))?;

    if paragraph.prev_sibling().is_none() {
        lift(tr, paragraph_id, LiftDirection::Front)
    } else if paragraph.next_sibling().is_none() && paragraph.first_child().is_none() {
        lift(tr, paragraph_id, LiftDirection::End)
    } else {
        Ok(false)
    }
}

fn lift(tr: &mut Transaction, paragraph_id: NodeId, direction: LiftDirection) -> CommandResult {
    let doc = tr.doc();
    let paragraph = doc
        .node(paragraph_id)
        .ok_or(CommandError::NodeNotFound(paragraph_id))?;

    let wrapper = match paragraph.parent() {
        Some(parent) if !matches!(parent.node(), Node::Root(_)) => parent,
        _ => return Ok(false),
    };

    if wrapper.spec().isolating {
        return Ok(false);
    }

    let wrapper_id = wrapper.id();
    let wrapper_parent = wrapper.parent().ok_or(CommandError::NoParent(wrapper_id))?;
    let wrapper_parent_id = wrapper_parent.id();

    let wrapper_index = wrapper
        .index()
        .ok_or(CommandError::Corrupted("wrapper has no index".into()))?;

    let target_index = match direction {
        LiftDirection::Front => wrapper_index,
        LiftDirection::End => wrapper_index + 1,
    };

    let mut children_types: Vec<NodeType> =
        wrapper_parent.children().map(|c| c.as_type()).collect();
    children_types.insert(target_index, NodeType::Paragraph);
    if !wrapper_parent
        .spec()
        .content
        .matches_sequence(&children_types)
    {
        return Ok(false);
    }

    tr.batch::<_, CommandError>(|tr| {
        tr.move_node(paragraph_id, wrapper_parent_id, target_index)?;

        let doc = tr.doc();
        if let Some(wrapper) = doc.node(wrapper_id) {
            let remaining: Vec<NodeType> = wrapper.children().map(|c| c.as_type()).collect();

            if wrapper.entry().children.is_empty() {
                tr.apply_steps(prune(&wrapper))?;
            } else if !wrapper.spec().content.matches_sequence(&remaining) {
                tr.apply_steps(dissolve(&wrapper))?;
            }
        }
        Ok(())
    })?;

    let doc = tr.doc();
    let paragraph = doc
        .node(paragraph_id)
        .ok_or(CommandError::NodeNotFound(paragraph_id))?;

    let new_selection = match paragraph.first_child() {
        Some(child) if matches!(child.node(), Node::Text(_)) => Selection::collapsed(Position {
            node_id: child.id(),
            offset: 0,
            affinity: Affinity::Downstream,
        }),
        _ => Selection::collapsed(Position {
            node_id: paragraph_id,
            offset: 0,
            affinity: Affinity::Downstream,
        }),
    };
    tr.set_selection(new_selection)?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote { paragraph { t1: text("A") } }
                    paragraph {}
                }
            }
            selection: (t1, 0) -> (t1, 1)
        };
        transact_fail!(initial, |tr| lift_paragraph(&mut tr));
    }

    #[test]
    fn not_at_start_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote { paragraph { t1: text("A") } }
                    paragraph {}
                }
            }
            selection: (t1, 1)
        };
        transact_fail!(initial, |tr| lift_paragraph(&mut tr));
    }

    #[test]
    fn not_first_child_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("A") }
                        paragraph { t1: text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| lift_paragraph(&mut tr));
    }

    #[test]
    fn parent_is_root_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| lift_paragraph(&mut tr));
    }

    #[test]
    fn list_item_filtered_by_content_spec() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("A") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| lift_paragraph(&mut tr));
    }

    #[test]
    fn lift_from_blockquote_multiple_children() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { t1: text("A") }
                        paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    blockquote {
                        paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_from_callout() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout {
                        paragraph { t1: text("A") }
                        paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    callout {
                        paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sole_child_prunes_wrapper() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { t1: text("A") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_empty_paragraph() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        p1: paragraph {}
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {}
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn cursor_on_text_node_at_start() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { t1: text("Hello") }
                        paragraph { text("World") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    blockquote {
                        paragraph { text("World") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_end_non_empty_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("A") }
                        paragraph { t1: text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| lift_paragraph(&mut tr));
    }

    #[test]
    fn lift_end_not_last_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("A") }
                        p1: paragraph {}
                        paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| lift_paragraph(&mut tr));
    }

    #[test]
    fn lift_end_isolating_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("Title") }
                        fold_content {
                            paragraph { text("A") }
                            p1: paragraph {}
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| lift_paragraph(&mut tr));
    }

    #[test]
    fn lift_end_content_spec_mismatch_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            p1: paragraph {}
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| lift_paragraph(&mut tr));
    }

    #[test]
    fn lift_end_empty_last_paragraph() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("A") }
                        p1: paragraph {}
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("A") }
                    }
                    p1: paragraph {}
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_end_from_callout() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout {
                        paragraph { text("A") }
                        p1: paragraph {}
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    callout {
                        paragraph { text("A") }
                    }
                    p1: paragraph {}
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_end_sole_child_uses_front() {
        // prev_sibling check takes priority, so sole child always lifts front
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        p1: paragraph {}
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {}
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
