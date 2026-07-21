use editor_crdt::Dot;
use editor_model::{DocView, NodeType, NodeView};
use editor_state::{GapCursor, Selection, as_gap_cursor};

use super::judge_outdent_list;
use crate::helpers::{
    collect_existing_lists_in_range, collect_list_items_for_kind_toggle,
    collect_listable_wrap_runs, contains_selected_plain_paragraph, find_enclosing_list_id,
    find_enclosing_list_item_id, is_block_container, is_list_type, list_item_parent_list_id,
    retain_topmost_list_items,
};
use crate::types::{LiftOfKindPlan, Verdict};

/// `Change` ⇒ `set_list_kind` 실행이 kind 교체 또는 리스트 wrap을 방출.
/// range의 wrap run은 사전 상태 기준이다 — kind 교체(1상)가 문단 run을 만들거나
/// 파괴하지 않는다는 성질에 의존하며, 차등 게이트가 이를 검증한다.
pub(crate) fn judge_set_list_kind(
    view: &DocView,
    selection: &Selection,
    target_list_type: NodeType,
) -> Verdict<()> {
    if !is_list_type(target_list_type) {
        return Verdict::NotApplicable;
    }
    if selection.anchor == selection.head {
        if let Some(list_id) = find_enclosing_list_id(view, selection.head.node) {
            let differs = view
                .node(list_id)
                .is_some_and(|list| list.node_type() != target_list_type);
            return if differs {
                Verdict::Change(())
            } else {
                Verdict::NotApplicable
            };
        }
        return judge_collapsed_wrap(view, selection.head.node, target_list_type);
    }

    let Some(rs) = selection.resolve(view) else {
        return Verdict::NotApplicable;
    };
    let kind_changes = collect_existing_lists_in_range(&rs).into_iter().any(|id| {
        view.node(id)
            .is_some_and(|list| list.node_type() != target_list_type)
    });
    if kind_changes {
        return Verdict::Change(());
    }
    if collect_listable_wrap_runs(&rs, target_list_type).is_empty() {
        Verdict::NotApplicable
    } else {
        Verdict::Change(())
    }
}

fn judge_collapsed_wrap(view: &DocView, cursor: Dot, target_list_type: NodeType) -> Verdict<()> {
    let mut node = view.node(cursor);
    let paragraph = loop {
        let Some(n) = node else {
            return Verdict::NotApplicable;
        };
        if n.node_type() == NodeType::Paragraph {
            break n;
        }
        node = n.parent();
    };
    let Some(parent) = paragraph.parent() else {
        return Verdict::NotApplicable;
    };
    if parent.node_type() == NodeType::ListItem || !parent.spec().content.matches(target_list_type)
    {
        return Verdict::NotApplicable;
    }
    Verdict::Change(())
}

/// `Change(plan)` ⇒ 선택 전체가 target kind 리스트 항목이라 lift가 관측 가능한
/// 구조 변화를 방출. range 경로는 `AbsorbOnly`를 내지 않는다 — collapsed 경로가
/// `judge_outdent_list`에 위임할 때만 그 verdict가 그대로 전달될 수 있다.
pub(crate) fn judge_lift_list_items_of_kind(
    view: &DocView,
    selection: &Selection,
    target_list_type: NodeType,
) -> Verdict<LiftOfKindPlan> {
    if !is_list_type(target_list_type) {
        return Verdict::NotApplicable;
    }
    if selection.anchor == selection.head {
        let is_target = find_enclosing_list_item_id(view, selection.head.node)
            .and_then(|item| list_item_parent_list_id(view, item))
            .and_then(|list| view.node(list))
            .is_some_and(|list| list.node_type() == target_list_type);
        if !is_target {
            return Verdict::NotApplicable;
        }
        return match judge_outdent_list(view, selection) {
            Verdict::Change(plan) => Verdict::Change(LiftOfKindPlan { items: plan.items }),
            Verdict::AbsorbOnly => Verdict::AbsorbOnly,
            Verdict::NotApplicable => Verdict::NotApplicable,
        };
    }

    let Some(rs) = selection.resolve(view) else {
        return Verdict::NotApplicable;
    };
    let mut items = collect_list_items_for_kind_toggle(&rs, view);
    let all_target = !items.is_empty()
        && !contains_selected_plain_paragraph(&rs, view)
        && items.iter().all(|item| {
            list_item_parent_list_id(view, *item)
                .and_then(|list| view.node(list))
                .is_some_and(|list| list.node_type() == target_list_type)
        });
    if !all_target {
        return Verdict::NotApplicable;
    }
    retain_topmost_list_items(view, &mut items);
    if items.is_empty() {
        return Verdict::NotApplicable;
    }
    Verdict::Change(LiftOfKindPlan { items })
}

/// handler의 first!(lift, set) 순서와 gap materialize 프렐류드를 미러링한다.
/// 실행은 이 합성을 소비하지 않으므로(각 half가 자기 판정을 소비), 미러 정합은
/// 차등 게이트가 보증한다.
pub fn judge_toggle_list_kind(
    view: &DocView,
    selection: &Selection,
    target_list_type: NodeType,
) -> Verdict<()> {
    if let Some(rs) = selection.resolve(view)
        && let Some(gap) = as_gap_cursor(&rs)
    {
        let parent = match &gap {
            GapCursor::BetweenMonolithic { parent, .. } => parent,
            GapCursor::IsolatingBoundary { host, .. } => host,
        };
        if is_block_container(parent) && parent.spec().content.matches(NodeType::Paragraph) {
            return judge_gap_toggle_list_kind(parent, target_list_type);
        }
    }
    match judge_lift_list_items_of_kind(view, selection, target_list_type) {
        Verdict::NotApplicable => judge_set_list_kind(view, selection, target_list_type),
        Verdict::Change(_) => Verdict::Change(()),
        Verdict::AbsorbOnly => Verdict::AbsorbOnly,
    }
}

fn judge_gap_toggle_list_kind(parent: &NodeView, target_list_type: NodeType) -> Verdict<()> {
    for ancestor in parent.ancestors() {
        if ancestor.node_type() == NodeType::ListItem {
            return if ancestor
                .parent()
                .is_some_and(|list| is_list_type(list.node_type()))
            {
                Verdict::Change(())
            } else {
                Verdict::NotApplicable
            };
        }
    }
    if parent.spec().content.matches(target_list_type) {
        Verdict::Change(())
    } else {
        Verdict::NotApplicable
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::NodeType;

    use super::*;

    #[test]
    fn judge_set_collapsed_in_other_kind_list_is_change() {
        let (state, ..) = state! {
            doc {
                root {
                    ordered_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let selection = state.selection.unwrap();
        let view = state.view();
        assert!(judge_set_list_kind(&view, &selection, NodeType::BulletList).changes());
    }

    #[test]
    fn judge_set_collapsed_plain_paragraph_is_change() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("A") } paragraph {} } }
            selection: (p1, 0)
        };
        let selection = state.selection.unwrap();
        let view = state.view();
        assert!(judge_set_list_kind(&view, &selection, NodeType::BulletList).changes());
    }

    #[test]
    fn judge_set_same_kind_collapsed_is_not_applicable() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let selection = state.selection.unwrap();
        let view = state.view();
        assert!(matches!(
            judge_set_list_kind(&view, &selection, NodeType::BulletList),
            crate::Verdict::NotApplicable
        ));
    }

    #[test]
    fn judge_toggle_same_kind_is_change_via_lift() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let selection = state.selection.unwrap();
        let view = state.view();
        assert!(judge_toggle_list_kind(&view, &selection, NodeType::BulletList).changes());
    }

    #[test]
    fn judge_toggle_mixed_plain_and_list_range_is_change_via_set() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    bullet_list { list_item { p2: paragraph { text("B") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let selection = state.selection.unwrap();
        let view = state.view();
        assert!(judge_toggle_list_kind(&view, &selection, NodeType::BulletList).changes());
    }

    #[test]
    fn judge_toggle_gap_cursor_at_root_is_change() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let selection = state.selection.unwrap();
        let view = state.view();
        assert!(judge_toggle_list_kind(&view, &selection, NodeType::BulletList).changes());
    }
}
