use hashbrown::HashSet;

use editor_crdt::Dot;
use editor_model::{DocView, NodeType, NodeView};
use editor_state::ResolvedSelection;
use editor_transaction::Transaction;

use crate::helpers::{
    find_enclosing_list_item_id, is_list_type, lift_list_items, lift_selected_list_items,
    list_item_own_paragraph_intersects, list_item_parent_list_id,
};
use crate::{CommandError, CommandResult};

pub fn lift_list_items_of_kind(tr: &mut Transaction, target_list_type: NodeType) -> CommandResult {
    if !is_list_type(target_list_type) {
        return Ok(false);
    }
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    if selection.is_collapsed() {
        let is_target = {
            let view = tr.view();
            find_enclosing_list_item_id(&view, selection.head.node)
                .and_then(|item| list_item_parent_list_id(&view, item))
                .and_then(|list| view.node(list))
                .is_some_and(|list| list.node_type() == target_list_type)
        };
        if !is_target {
            return Ok(false);
        }
        return lift_selected_list_items(tr);
    }

    let all_target = {
        let view = tr.view();
        let resolved = selection
            .resolve(&view)
            .ok_or_else(|| CommandError::Corrupted("cannot resolve list selection".into()))?;
        let items = collect_list_items_for_kind_toggle(&resolved, &view);
        !items.is_empty()
            && !contains_selected_plain_paragraph(&resolved, &view)
            && items.iter().all(|item| {
                list_item_parent_list_id(&view, *item)
                    .and_then(|list| view.node(list))
                    .is_some_and(|list| list.node_type() == target_list_type)
            })
    };
    if !all_target {
        return Ok(false);
    }
    let items = {
        let view = tr.view();
        let resolved = selection
            .resolve(&view)
            .ok_or_else(|| CommandError::Corrupted("cannot resolve list selection".into()))?;
        collect_list_items_for_kind_toggle(&resolved, &view)
    };
    lift_list_items(tr, items)
}

fn collect_list_items_for_kind_toggle(
    selection: &ResolvedSelection<'_>,
    view: &DocView,
) -> Vec<Dot> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    if let Some(root) = view.root() {
        collect_list_items_for_kind_toggle_in(selection, &root, &mut items, &mut seen);
    }
    items
}

fn collect_list_items_for_kind_toggle_in(
    selection: &ResolvedSelection<'_>,
    node: &NodeView<'_>,
    out: &mut Vec<Dot>,
    seen: &mut HashSet<Dot>,
) {
    if !selection.intersects_subtree(node) {
        return;
    }
    if selection.contains_subtree(node) && is_unsupported_whole_container(node.node_type()) {
        return;
    }

    if node.node_type() == NodeType::ListItem
        && list_item_own_paragraph_intersects(selection, node)
        && seen.insert(node.id())
    {
        out.push(node.id());
    }

    for child in node.child_blocks() {
        collect_list_items_for_kind_toggle_in(selection, &child, out, seen);
    }
}

fn is_unsupported_whole_container(node_type: NodeType) -> bool {
    !matches!(
        node_type,
        NodeType::Root
            | NodeType::Paragraph
            | NodeType::ListItem
            | NodeType::BulletList
            | NodeType::OrderedList
    )
}

fn contains_selected_plain_paragraph(
    selection: &editor_state::ResolvedSelection<'_>,
    view: &DocView,
) -> bool {
    view.root()
        .is_some_and(|root| contains_plain_paragraph_in(selection, &root))
}

fn contains_plain_paragraph_in(
    selection: &editor_state::ResolvedSelection<'_>,
    node: &NodeView<'_>,
) -> bool {
    if !selection.intersects_subtree(node) {
        return false;
    }
    if node.node_type() == NodeType::Paragraph {
        return !node
            .ancestors()
            .skip(1)
            .any(|ancestor| ancestor.node_type() == NodeType::ListItem);
    }

    node.child_blocks().any(|child| {
        if selection.contains_subtree(&child) && is_unsupported_whole_container(child.node_type()) {
            return false;
        }
        contains_plain_paragraph_in(selection, &child)
    })
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::NodeType;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn collapsed_target_kind_lifts_current_item() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_items_of_kind(
            &mut tr,
            NodeType::BulletList,
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("A") } paragraph {} } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn middle_target_kind_item_lifts_and_splits_list() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p2: paragraph { text("B") } }
                        list_item { paragraph { text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_items_of_kind(
            &mut tr,
            NodeType::BulletList,
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { text("A") } } }
                    p2: paragraph { text("B") }
                    bullet_list { list_item { paragraph { text("C") } } }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn different_kind_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    ordered_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        transact_fail!(initial, |tr| lift_list_items_of_kind(
            &mut tr,
            NodeType::BulletList,
        ));
    }

    #[test]
    fn mixed_plain_paragraph_and_target_list_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    bullet_list { list_item { p2: paragraph { text("B") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        transact_fail!(initial, |tr| lift_list_items_of_kind(
            &mut tr,
            NodeType::BulletList,
        ));
    }

    #[test]
    fn fully_selected_unsupported_wrapper_keeps_internal_target_list_unchanged() {
        let (initial, _root, ..) = state! {
            doc {
                root: root {
                    blockquote {
                        bullet_list {
                            list_item { p1: paragraph { text("A") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (root, 0, >) -> (root, 1, <)
        };
        let expected = initial.clone();

        let (actual, steps, ..) = transact_fail!(initial, |tr| lift_list_items_of_kind(
            &mut tr,
            NodeType::BulletList,
        ));

        assert!(steps.is_empty());
        assert_state_eq!(&actual, &expected);
    }
}
