use editor_crdt::Dot;
use editor_model::{AtomLeaf, ChildView, DocView};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{child_elem_id, prune_empty_full, remove_subtree_full};
use crate::{CommandError, CommandResult};

fn child_is_unit(child: &ChildView) -> bool {
    match child {
        ChildView::Block(b) => b.spec().is_unit(),
        ChildView::Leaf(l) => l.as_atom().is_some_and(AtomLeaf::is_block_level),
    }
}

fn child_index(view: &DocView, parent: Dot, child: Dot) -> Option<usize> {
    view.node(parent)?
        .children()
        .position(|c| child_elem_id(&c) == child)
}

pub fn select_node_forward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;
    let view = tr.state().view();
    let start = view
        .node(pos.node)
        .ok_or(CommandError::NodeNotFound(pos.node))?;

    if pos.offset != start.children().count() {
        return Ok(false);
    }

    let mut current = start;
    let (parent_id, next_elem) = loop {
        let Some(parent) = current.parent() else {
            return Ok(false);
        };
        let idx = current
            .index()
            .ok_or_else(|| CommandError::orphan_child(current.id(), parent.id()))?;
        match parent.child_at(idx + 1) {
            Some(next) => {
                if !child_is_unit(&next) {
                    return Ok(false);
                }
                break (parent.id(), child_elem_id(&next));
            }
            None => current = parent,
        }
    };

    let remove_start = !start.spec().is_leaf() && start.children().next().is_none();
    let start_id = start.id();
    let start_parent_id = start.parent().map(|p| p.id());

    if remove_start && let Some(pid) = start_parent_id {
        // Capture the ancestor chain before removal: prune severs parent links,
        // so we cannot walk upward afterward to repair surviving ancestors.
        let ancestor_chain: Vec<Dot> = {
            let view = tr.state().view();
            let mut chain = Vec::new();
            let mut cur = Some(pid);
            while let Some(id) = cur {
                let parent = view.node(id).and_then(|n| n.parent()).map(|p| p.id());
                chain.push(id);
                cur = parent;
            }
            chain
        };

        tr.batch::<_, CommandError>(|tr| {
            remove_subtree_full(tr, start_id)?;

            prune_empty_full(tr, pid)?;

            // prune may have cascaded removals; re-fulfill the surviving
            // ancestors so structural containers and the root's trailing
            // paragraph requirement stay schema-valid.
            for id in &ancestor_chain {
                let steps = {
                    let view = tr.state().view();
                    view.node(*id).map(|node| fulfill(&node))
                };
                if let Some(steps) = steps {
                    tr.apply_steps(steps)?;
                }
            }

            Ok(())
        })?;
    }

    let next_idx = {
        let view = tr.state().view();
        child_index(&view, parent_id, next_elem)
            .ok_or_else(|| CommandError::orphan_child(next_elem, parent_id))?
    };

    tr.set_selection(Some(Selection::new(
        Position {
            node: parent_id,
            offset: next_idx + 1,
            affinity: Affinity::Upstream,
        },
        Position {
            node: parent_id,
            offset: next_idx,
            affinity: Affinity::Downstream,
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
            doc { root { p1: paragraph { text("Hello") } horizontal_rule paragraph } }
            selection: (p1, 0) -> (p1, 1)
        };
        transact_fail!(initial, |tr| select_node_forward(&mut tr));
    }

    #[test]
    fn rejects_if_not_on_last_position() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } horizontal_rule paragraph } }
            selection: (p1, 3)
        };
        transact_fail!(initial, |tr| select_node_forward(&mut tr));
    }

    #[test]
    fn rejects_if_next_sibling_is_not_leaf_or_monolithic() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } paragraph } }
            selection: (p1, 5)
        };
        transact_fail!(initial, |tr| select_node_forward(&mut tr));
    }

    #[test]
    fn select_node_forward_on_last_position() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } horizontal_rule paragraph } }
            selection: (p1, 5)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_forward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root { paragraph { text("hello") } horizontal_rule paragraph } }
            selection: (r, 2, <) -> (r, 1, >)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_forward_removes_empty_callout_when_its_only_paragraph_is_emptied() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout { p1: paragraph {} }
                    horizontal_rule
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_forward(&mut tr));

        let (expected, ..) = state! {
            doc {
                r1: root {
                    horizontal_rule
                    paragraph {}
                }
            }
            selection: (r1, 1, <) -> (r1, 0, >)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_forward_removes_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { paragraph { text("hello") } p: paragraph horizontal_rule paragraph } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_forward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root { paragraph { text("hello") } horizontal_rule paragraph } }
            selection: (r, 2, <) -> (r, 1, >)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_forward_on_fold() {
        let (initial, ..) = state! {
            doc { root {
                p: paragraph {}
                fold { fold_title { text("t") } fold_content { paragraph { text("c") } } }
                paragraph {}
            } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_forward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root {
                fold { fold_title { text("t") } fold_content { paragraph { text("c") } } }
                paragraph {}
            } }
            selection: (r, 1, <) -> (r, 0, >)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_forward_on_callout() {
        let (initial, ..) = state! {
            doc { root {
                p: paragraph {}
                callout { paragraph { text("c") } }
                paragraph {}
            } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_forward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root {
                callout { paragraph { text("c") } }
                paragraph {}
            } }
            selection: (r, 1, <) -> (r, 0, >)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_forward_on_blockquote() {
        let (initial, ..) = state! {
            doc { root {
                p: paragraph {}
                blockquote { paragraph { text("c") } }
                paragraph {}
            } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_forward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root {
                blockquote { paragraph { text("c") } }
                paragraph {}
            } }
            selection: (r, 1, <) -> (r, 0, >)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_forward_on_table() {
        let (initial, ..) = state! {
            doc { root {
                p: paragraph {}
                table { table_row { table_cell { paragraph { text("c") } } } }
                paragraph {}
            } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_forward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root {
                table { table_row { table_cell { paragraph { text("c") } } } }
                paragraph {}
            } }
            selection: (r, 1, <) -> (r, 0, >)
        };

        assert_state_eq!(actual, expected);
    }
}
