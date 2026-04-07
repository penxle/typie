use editor_model::{Doc, Node, NodeId, NodeRef, NodeType};
use editor_schema::NodeSpecExt;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::{CommandError, CommandResult};

pub fn sink_paragraph_backward(tr: &mut Transaction) -> CommandResult {
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

    let prev = match paragraph.prev_sibling() {
        Some(prev) => prev,
        None => return Ok(false),
    };

    if matches!(prev.node(), Node::Paragraph(_)) {
        return Ok(false);
    }

    let target_id = match find_sink_target(&doc, &prev, &paragraph) {
        Some(id) => id,
        None => return Ok(false),
    };

    let source_parent_id = paragraph
        .parent()
        .ok_or(CommandError::NoParent(paragraph_id))?
        .id();

    let doc = tr.doc();
    let target = doc
        .node(target_id)
        .ok_or(CommandError::NodeNotFound(target_id))?;
    let target_children_count = target.entry().children.len();

    tr.batch::<_, CommandError>(|tr| {
        tr.move_node(paragraph_id, target_id, target_children_count)?;
        let doc = tr.doc();
        if let Some(parent) = doc.node(source_parent_id) {
            tr.apply_steps(fulfill(&parent))?;
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

fn find_sink_target(doc: &Doc, start: &NodeRef, paragraph: &NodeRef) -> Option<NodeId> {
    let mut candidate = None;
    let mut current_id = start.id();

    loop {
        let current = doc.node(current_id)?;
        let spec = current.spec();

        if spec.isolating {
            break;
        }

        // Content check: can Paragraph be appended to this node's children?
        let mut children_types: Vec<NodeType> =
            current.children().map(|child| child.as_type()).collect();
        children_types.push(NodeType::Paragraph);

        let content_ok = spec.content.matches_sequence(&children_types);

        // Context check: Paragraph and all its descendants valid in new location?
        let context_ok = if content_ok {
            let new_base: Vec<NodeType> = build_ancestor_path(doc, current_id);
            validate_context_deep(doc, paragraph, &new_base)
        } else {
            false
        };

        if content_ok && context_ok {
            candidate = Some(current_id);
        }

        match current.last_child() {
            Some(child) => current_id = child.id(),
            None => break,
        }
    }

    candidate
}

/// Build ancestor path from root to node (inclusive), as NodeType list.
fn build_ancestor_path(doc: &Doc, node_id: NodeId) -> Vec<NodeType> {
    let mut path = Vec::new();
    let mut current = node_id;
    while let Some(node) = doc.node(current) {
        path.push(node.as_type());
        match node.entry().parent {
            Some(parent_id) => current = parent_id,
            None => break,
        }
    }
    path.reverse();
    path
}

/// Validate context for a node and all its descendants at a hypothetical new base path.
fn validate_context_deep(doc: &Doc, node: &NodeRef, base_path: &[NodeType]) -> bool {
    let mut stack: Vec<(NodeId, Vec<NodeType>)> = vec![(node.id(), {
        let mut p = base_path.to_vec();
        p.push(node.as_type());
        p
    })];

    while let Some((node_id, path)) = stack.pop() {
        let Some(current) = doc.node(node_id) else {
            return false;
        };

        if !current.spec().context.matches(&path) {
            return false;
        }

        for child in current.children() {
            let mut child_path = path.clone();
            child_path.push(child.as_type());
            stack.push((child.id(), child_path));
        }
    }

    true
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
                    blockquote { paragraph { t0: text("A") } }
                    paragraph { t1: text("B") }
                }
            }
            selection: (t1, 0) -> (t1, 1)
        };
        transact_fail!(initial, |tr| sink_paragraph_backward(&mut tr));
    }

    #[test]
    fn sink_into_blockquote() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote { paragraph { t1: text("A") } }
                    paragraph { t2: text("B") }
                }
            }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { t1: text("A") }
                        paragraph { t2: text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_into_callout() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout { paragraph { t1: text("A") } }
                    paragraph { t2: text("B") }
                }
            }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    callout {
                        paragraph { t1: text("A") }
                        paragraph { t2: text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_deep_blockquote() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        blockquote { paragraph { t1: text("A") } }
                    }
                    paragraph { t2: text("B") }
                }
            }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        blockquote {
                            paragraph { t1: text("A") }
                            paragraph { t2: text("B") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn no_prev_sibling_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| sink_paragraph_backward(&mut tr));
    }

    #[test]
    fn prev_is_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    paragraph { t2: text("B") }
                }
            }
            selection: (t2, 0)
        };
        transact_fail!(initial, |tr| sink_paragraph_backward(&mut tr));
    }

    #[test]
    fn prev_list_does_not_sink_into_list_item() {
        // ListItem content: Paragraph, (BulletList|OrderedList)*
        // Appending Paragraph at end is invalid
        // But BulletList itself accepts ListItem+, not Paragraph
        // So no valid target → Ok(false)
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("A") } }
                    }
                    paragraph { t2: text("B") }
                }
            }
            selection: (t2, 0)
        };
        transact_fail!(initial, |tr| sink_paragraph_backward(&mut tr));
    }

    #[test]
    fn not_at_paragraph_start_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote { paragraph { t1: text("A") } }
                    paragraph { t2: text("B") }
                }
            }
            selection: (t2, 1)
        };
        transact_fail!(initial, |tr| sink_paragraph_backward(&mut tr));
    }

    #[test]
    fn sink_empty_paragraph_into_blockquote() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote { paragraph { t1: text("A") } }
                    p2: paragraph {}
                }
            }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { t1: text("A") }
                        p2: paragraph {}
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
