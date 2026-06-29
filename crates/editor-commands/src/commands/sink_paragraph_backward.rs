use editor_crdt::Dot;
use editor_model::{ChildView, DocView, NodeType, NodeView};
use editor_state::last_cursor_position;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{child_node_type, prev_sibling};
use crate::{CommandError, CommandResult};

pub fn sink_paragraph_backward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;
    if pos.offset > 0 {
        return Ok(false);
    }
    let paragraph_id = pos.node;

    let (is_empty, target_id, source_parent_id) = {
        let view = tr.state().view();
        let paragraph = view
            .node(paragraph_id)
            .ok_or(CommandError::NodeNotFound(paragraph_id))?;

        if paragraph.node_type() != NodeType::Paragraph {
            return Ok(false);
        }

        let is_empty = paragraph.children().next().is_none();

        let prev = match prev_sibling(&paragraph) {
            Some(ChildView::Block(b)) => b,
            _ => return Ok(false),
        };

        if prev.node_type() == NodeType::Paragraph {
            return Ok(false);
        }

        let target_id = match find_sink_target(&view, &prev, &paragraph) {
            Some(id) => id,
            None => return Ok(false),
        };

        let source_parent_id = paragraph
            .parent()
            .ok_or(CommandError::NoParent(paragraph_id))?
            .id();

        (is_empty, target_id, source_parent_id)
    };

    if is_empty {
        let cursor_pos = {
            let view = tr.state().view();
            let target = view
                .node(target_id)
                .ok_or(CommandError::NodeNotFound(target_id))?;
            last_cursor_position(&target).ok_or(CommandError::NodeNotFound(target_id))?
        };

        tr.batch::<_, CommandError>(|tr| {
            tr.remove_subtree(paragraph_id)?;
            let steps = {
                let view = tr.state().view();
                view.node(source_parent_id)
                    .map(|parent| fulfill(&parent))
                    .unwrap_or_default()
            };
            tr.apply_steps(steps)?;
            Ok(())
        })?;

        tr.set_selection(Some(Selection::collapsed(Position {
            node: cursor_pos.node,
            offset: cursor_pos.offset,
            affinity: Affinity::Downstream,
        })))?;
    } else {
        let target_children_count = {
            let view = tr.state().view();
            view.node(target_id)
                .ok_or(CommandError::NodeNotFound(target_id))?
                .children()
                .count()
        };

        let mut moved_id = paragraph_id;
        tr.batch::<_, CommandError>(|tr| {
            tr.move_node(paragraph_id, target_id, target_children_count)?;
            moved_id = {
                let view = tr.state().view();
                view.node(target_id)
                    .and_then(|target| target.child_blocks().last().map(|b| b.id()))
                    .ok_or(CommandError::NodeNotFound(target_id))?
            };
            let steps = {
                let view = tr.state().view();
                view.node(source_parent_id)
                    .map(|parent| fulfill(&parent))
                    .unwrap_or_default()
            };
            tr.apply_steps(steps)?;
            Ok(())
        })?;

        tr.set_selection(Some(Selection::collapsed(Position {
            node: moved_id,
            offset: 0,
            affinity: Affinity::Downstream,
        })))?;
    }

    Ok(true)
}

fn find_sink_target(view: &DocView, start: &NodeView, paragraph: &NodeView) -> Option<Dot> {
    let mut candidate = None;
    let mut current_id = start.id();

    loop {
        let current = view.node(current_id)?;
        let spec = current.spec();

        if spec.isolating {
            break;
        }

        // Content check: can Paragraph be appended to this node's children?
        let mut children_types: Vec<NodeType> =
            current.children().map(|c| child_node_type(&c)).collect();
        children_types.push(NodeType::Paragraph);

        let content_ok = spec.content.matches_sequence(&children_types);

        // Context check: Paragraph and all its descendants valid in new location?
        let context_ok = if content_ok {
            let new_base: Vec<NodeType> = build_ancestor_path(view, current_id);
            validate_context_deep(view, paragraph, &new_base)
        } else {
            false
        };

        if content_ok && context_ok {
            candidate = Some(current_id);
        }

        match current.last_child() {
            Some(ChildView::Block(b)) => current_id = b.id(),
            _ => break,
        }
    }

    candidate
}

/// Build ancestor path from root to node (inclusive), as NodeType list.
fn build_ancestor_path(view: &DocView, node_id: Dot) -> Vec<NodeType> {
    let mut path = Vec::new();
    let mut current = node_id;
    while let Some(node) = view.node(current) {
        path.push(node.node_type());
        match node.parent() {
            Some(parent) => current = parent.id(),
            None => break,
        }
    }
    path.reverse();
    path
}

/// Validate context for a node and all its descendants at a hypothetical new base path.
fn validate_context_deep(view: &DocView, node: &NodeView, base_path: &[NodeType]) -> bool {
    let mut stack: Vec<(Dot, Vec<NodeType>)> = vec![(node.id(), {
        let mut p = base_path.to_vec();
        p.push(node.node_type());
        p
    })];

    while let Some((node_id, path)) = stack.pop() {
        let Some(current) = view.node(node_id) else {
            return false;
        };

        if !current.spec().context.matches(&path) {
            return false;
        }

        for child in current.children() {
            if let ChildView::Block(b) = child {
                let mut child_path = path.clone();
                child_path.push(b.node_type());
                stack.push((b.id(), child_path));
            }
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
                    blockquote { p1: paragraph { text("A") } }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p2, 0) -> (p2, 1)
        };
        transact_fail!(initial, |tr| sink_paragraph_backward(&mut tr));
    }

    #[test]
    fn sink_into_blockquote() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote { p1: paragraph { text("A") } }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        p1: paragraph { text("A") }
                        p2: paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_into_callout() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout { p1: paragraph { text("A") } }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    callout {
                        p1: paragraph { text("A") }
                        p2: paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_deep_blockquote() {
        // Blockquote/Callout are monolithic and cannot nest, and the only
        // containers that accept a blockquote (FoldContent/TableCell) are
        // isolating, so the deepest reachable sink target is the blockquote
        // itself. This exercises descend-and-backtrack: the sink walks past the
        // non-appendable inner list and lands in the blockquote.
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        bullet_list { list_item { p1: paragraph { text("A") } } }
                    }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        bullet_list { list_item { p1: paragraph { text("A") } } }
                        p2: paragraph { text("B") }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn no_prev_sibling_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| sink_paragraph_backward(&mut tr));
    }

    #[test]
    fn prev_is_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p2, 0)
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
                        list_item { p1: paragraph { text("A") } }
                    }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p2, 0)
        };
        transact_fail!(initial, |tr| sink_paragraph_backward(&mut tr));
    }

    #[test]
    fn not_at_paragraph_start_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote { p1: paragraph { text("A") } }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p2, 1)
        };
        transact_fail!(initial, |tr| sink_paragraph_backward(&mut tr));
    }

    #[test]
    fn empty_paragraph_deleted_and_cursor_moves_to_target() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote { p1: paragraph { text("A") } }
                    p2: paragraph {}
                }
            }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote { p1: paragraph { text("A") } }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }
}
