use editor_model::{Node, NodeId};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill, prune};

use crate::{CommandError, CommandResult};

pub fn select_node_backward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    if pos.offset != 0 {
        return Ok(false);
    }

    let doc = tr.doc();
    let start = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let mut current = start;
    let prev = loop {
        if let Some(prev) = current.prev_sibling() {
            break prev;
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return Ok(false),
        }
    };

    if matches!(prev.node(), Node::Text(_)) || !prev.spec().is_unit() {
        return Ok(false);
    }

    let parent_id = prev.parent().ok_or(CommandError::NoParent(prev.id()))?.id();
    let prev_idx = prev
        .index()
        .ok_or_else(|| CommandError::orphan_child(prev.id(), parent_id))?;

    let remove_start = !start.spec().is_leaf() && start.entry().children.is_empty();
    let start_id = start.id();
    let start_parent_id = start.parent().map(|p| p.id());

    if remove_start && let Some(pid) = start_parent_id {
        // Capture the ancestor chain before removal: prune severs parent links,
        // so we cannot walk upward afterward to repair surviving ancestors.
        let ancestor_chain: Vec<NodeId> = {
            let doc = tr.doc();
            let mut chain = Vec::new();
            let mut current = Some(pid);
            while let Some(id) = current {
                chain.push(id);
                current = doc.node(id).and_then(|n| n.parent()).map(|p| p.id());
            }
            chain
        };

        tr.batch::<_, CommandError>(|tr| {
            tr.remove_subtree(start_id)?;

            let doc = tr.doc();
            if let Some(parent) = doc.node(pid)
                && parent.entry().children.is_empty()
                && !parent.spec().structural
            {
                tr.apply_steps(prune(&parent))?;
            }

            // prune may have cascaded removals; re-fulfill the surviving
            // ancestors so structural containers and the root's trailing
            // paragraph requirement stay schema-valid.
            for &id in &ancestor_chain {
                let doc = tr.doc();
                if let Some(node) = doc.node(id) {
                    tr.apply_steps(fulfill(&node))?;
                }
            }

            Ok(())
        })?;
    }

    tr.set_selection(Some(Selection::new(
        Position {
            node_id: parent_id,
            offset: prev_idx,
            affinity: Affinity::Downstream,
        },
        Position {
            node_id: parent_id,
            offset: prev_idx + 1,
            affinity: Affinity::Upstream,
        },
    )))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn rejects_range_selection() {
        let (initial, ..) = state! {
            doc { root { horizontal_rule paragraph { t: text("Hello") } } }
            selection: (t, 0) -> (t, 1)
        };
        transact_fail!(initial, |tr| select_node_backward(&mut tr));
    }

    #[test]
    fn rejects_if_not_on_first_position() {
        let (initial, ..) = state! {
            doc { root { horizontal_rule paragraph { t: text("hello") } } }
            selection: (t, 1)
        };
        transact_fail!(initial, |tr| select_node_backward(&mut tr));
    }

    #[test]
    fn rejects_if_prev_sibling_is_not_leaf_or_monolithic() {
        let (initial, ..) = state! {
            doc { root { paragraph paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        transact_fail!(initial, |tr| select_node_backward(&mut tr));
    }

    #[test]
    fn select_node_backward_on_first_position() {
        let (initial, ..) = state! {
            doc { root { horizontal_rule paragraph { t: text("hello") } } }
            selection: (t, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_backward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root { horizontal_rule paragraph { text("hello") } } }
            selection: (r, 0, >) -> (r, 1, <)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_backward_removes_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { horizontal_rule p: paragraph paragraph { text("hello") } } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_backward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root { horizontal_rule paragraph { text("hello") } } }
            selection: (r, 0, >) -> (r, 1, <)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_backward_removes_empty_callout_when_its_only_paragraph_is_emptied() {
        let (initial, ..) = state! {
            doc {
                root {
                    horizontal_rule
                    callout { p1: paragraph {} }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_backward(&mut tr));

        let (expected, ..) = state! {
            doc {
                r1: root {
                    horizontal_rule
                    paragraph {}
                }
            }
            selection: (r1, 0, >) -> (r1, 1, <)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_backward_removes_empty_callout_and_refulfills_root_trailing_paragraph() {
        let (initial, ..) = state! {
            doc {
                root {
                    horizontal_rule
                    callout { p1: paragraph {} }
                }
            }
            selection: (p1, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_backward(&mut tr));

        let (expected, ..) = state! {
            doc {
                r1: root {
                    horizontal_rule
                    paragraph {}
                }
            }
            selection: (r1, 0, >) -> (r1, 1, <)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_backward_removes_empty_paragraph_but_keeps_trailing_paragraph() {
        let (initial, ..) = state! {
            doc { root { horizontal_rule p: paragraph } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_backward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root { horizontal_rule paragraph } }
            selection: (r, 0, >) -> (r, 1, <)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_backward_on_fold() {
        let (initial, ..) = state! {
            doc { root {
                fold { fold_title { text("t") } fold_content { paragraph { text("c") } } }
                p: paragraph {}
            } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_backward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root {
                fold { fold_title { text("t") } fold_content { paragraph { text("c") } } }
                paragraph {}
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_backward_on_callout() {
        let (initial, ..) = state! {
            doc { root {
                callout { paragraph { text("c") } }
                p: paragraph {}
            } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_backward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root {
                callout { paragraph { text("c") } }
                paragraph {}
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_backward_on_blockquote() {
        let (initial, ..) = state! {
            doc { root {
                blockquote { paragraph { text("c") } }
                p: paragraph {}
            } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_backward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root {
                blockquote { paragraph { text("c") } }
                paragraph {}
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_backward_on_table() {
        let (initial, ..) = state! {
            doc { root {
                table { table_row { table_cell { paragraph { text("c") } } } }
                p: paragraph {}
            } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_backward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root {
                table { table_row { table_cell { paragraph { text("c") } } } }
                paragraph {}
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };

        assert_state_eq!(actual, expected);
    }
}
