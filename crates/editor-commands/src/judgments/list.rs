use editor_model::DocView;
use editor_state::{Selection, StableResolveCtx, StableSelection};
use editor_transaction::Transaction;

use crate::helpers::{
    collect_list_items_in_selection, find_enclosing_list_item_id, group_list_items_by_parent,
    is_materializable_synthetic, lift_list_item_inner, lift_list_items_planned,
    list_item_parent_list_id, retain_topmost_list_items, sink_list_item_inner,
};
use crate::types::{IndentPlan, OutdentPlan, Verdict};
use crate::{CommandError, CommandResult};

/// `Change` ⇒ `lift_selected_list_items` 실행 시 관측 가능한 구조 변화 보장.
/// `AbsorbOnly` ⇒ 실행이 handled(true)를 반환하지만 관측 변화는 없음.
pub fn judge_outdent_list(view: &DocView, selection: &Selection) -> Verdict<OutdentPlan> {
    if selection.anchor == selection.head {
        let Some(item) = find_enclosing_list_item_id(view, selection.head.node) else {
            return Verdict::NotApplicable;
        };
        if list_item_parent_list_id(view, item).is_none() {
            return Verdict::NotApplicable;
        }
        return Verdict::Change(OutdentPlan { items: vec![item] });
    }

    let Some(rs) = selection.resolve(view) else {
        return Verdict::NotApplicable;
    };
    let mut items = collect_list_items_in_selection(&rs);
    if items.is_empty() {
        return Verdict::NotApplicable;
    }
    retain_topmost_list_items(view, &mut items);
    if items.is_empty() {
        return Verdict::NotApplicable;
    }
    if !items
        .iter()
        .any(|item| list_item_parent_list_id(view, *item).is_some())
    {
        return Verdict::AbsorbOnly;
    }
    Verdict::Change(OutdentPlan { items })
}

pub(crate) fn lift_selected_list_items(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let verdict = {
        let view = tr.view();
        if selection.anchor != selection.head && selection.resolve(&view).is_none() {
            return Err(CommandError::Corrupted("cannot resolve selection".into()));
        }
        judge_outdent_list(&view, &selection)
    };
    match verdict {
        crate::types::Verdict::NotApplicable => Ok(false),
        crate::types::Verdict::AbsorbOnly => Ok(true),
        crate::types::Verdict::Change(plan) => {
            if selection.anchor == selection.head {
                lift_list_item_inner(tr, plan.items[0])
            } else {
                lift_list_items_planned(tr, plan.items)
            }
        }
    }
}

/// `Change` ⇒ 최소 한 그룹의 첫 항목이 선행 형제를 가져 sink가 실제로 일어남, 또는
/// 선택 endpoint가 synthetic scaffold라 handler의 materialize 프렐류드가 실 dot을
/// 남김(핸들러 chain은 이 verdict의 handled(true)를 소비하므로 롤백되지 않음).
/// `AbsorbOnly` ⇒ range 선택이 리스트 항목을 포함하지만 sink 가능한 그룹이 없고
/// endpoint도 모두 real dot — 실행은 handled(true)로 Tab 폴스루를 흡수하고 관측
/// 변화는 없음.
pub fn judge_indent_list(view: &DocView, selection: &Selection) -> Verdict<IndentPlan> {
    let Some(rs) = selection.resolve(view) else {
        return Verdict::NotApplicable;
    };
    let items = collect_list_items_in_selection(&rs);
    if items.is_empty() {
        return Verdict::NotApplicable;
    }
    let any_sinkable = group_list_items_by_parent(view, &items)
        .iter()
        .any(|group| {
            view.node(group.items[0])
                .and_then(|item| item.index())
                .map(|index| index > 0)
                .unwrap_or(false)
        });
    if any_sinkable {
        Verdict::Change(IndentPlan { items })
    } else if selection.is_collapsed() {
        Verdict::NotApplicable
    } else if is_materializable_synthetic(selection.anchor.node)
        || is_materializable_synthetic(selection.head.node)
    {
        Verdict::Change(IndentPlan { items })
    } else {
        Verdict::AbsorbOnly
    }
}

pub(crate) fn sink_selected_list_items(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let plan = {
        let view = tr.view();
        if selection.resolve(&view).is_none() {
            return Err(CommandError::Corrupted("cannot resolve selection".into()));
        }
        match judge_indent_list(&view, &selection) {
            crate::types::Verdict::NotApplicable => return Ok(false),
            crate::types::Verdict::AbsorbOnly => return Ok(true),
            crate::types::Verdict::Change(plan) => plan,
        }
    };
    let items = plan.items;

    let stable_selection = StableSelection::capture(&selection, &tr.view());
    let mut groups = {
        let view = tr.view();
        group_list_items_by_parent(&view, &items)
    };
    groups.sort_by(|a, b| {
        b.depth
            .cmp(&a.depth)
            .then_with(|| a.first_index.cmp(&b.first_index))
    });

    let mut any_sunk = false;
    for group in groups {
        let first_has_prev = {
            let view = tr.view();
            view.node(group.items[0])
                .and_then(|item| item.index())
                .map(|index| index > 0)
                .unwrap_or(false)
        };

        if !first_has_prev {
            continue;
        }

        for item_id in group.items {
            let exists = {
                let view = tr.view();
                view.node(item_id).is_some()
            };
            if !exists {
                continue;
            }
            let new_id = sink_list_item_inner(tr, item_id)?;
            if new_id.is_some() {
                any_sunk = true;
            }
        }
    }
    if !any_sunk {
        return Ok(!selection.is_collapsed());
    }

    let sel = {
        let view = tr.view();
        let ctx = StableResolveCtx::from_live(&view, tr.state().projected.seq_checkout());
        stable_selection.resolve(&ctx)
    }
    .ok_or_else(|| CommandError::Corrupted("cannot restore list selection".into()))?;
    tr.set_selection(Some(sel))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;

    #[test]
    fn judge_outdent_collapsed_in_list_is_change() {
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
        let verdict = judge_outdent_list(&view, &selection);
        assert!(matches!(verdict, crate::Verdict::Change(_)));
    }

    #[test]
    fn judge_outdent_outside_list_is_not_applicable() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("A") } } }
            selection: (p1, 0)
        };
        let selection = state.selection.unwrap();
        let view = state.view();
        let verdict = judge_outdent_list(&view, &selection);
        assert!(matches!(verdict, crate::Verdict::NotApplicable));
    }

    #[test]
    fn judge_outdent_range_plan_holds_topmost_items() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("A") }
                            bullet_list { list_item { p2: paragraph { text("B") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let selection = state.selection.unwrap();
        let view = state.view();
        match judge_outdent_list(&view, &selection) {
            crate::Verdict::Change(plan) => assert_eq!(plan.items.len(), 1),
            _ => panic!("expected Change"),
        }
    }

    #[test]
    fn judge_indent_second_item_is_change() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let selection = state.selection.unwrap();
        let view = state.view();
        assert!(matches!(
            judge_indent_list(&view, &selection),
            crate::Verdict::Change(_)
        ));
    }

    #[test]
    fn judge_indent_first_item_collapsed_is_not_applicable() {
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
            judge_indent_list(&view, &selection),
            crate::Verdict::NotApplicable
        ));
    }

    #[test]
    fn judge_indent_first_item_range_is_absorb_only() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("AB") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p1, 2)
        };
        let selection = state.selection.unwrap();
        let view = state.view();
        assert!(matches!(
            judge_indent_list(&view, &selection),
            crate::Verdict::AbsorbOnly
        ));
    }

    #[test]
    fn judge_indent_outside_list_is_not_applicable() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("A") } } }
            selection: (p1, 0)
        };
        let selection = state.selection.unwrap();
        let view = state.view();
        assert!(matches!(
            judge_indent_list(&view, &selection),
            crate::Verdict::NotApplicable
        ));
    }
}
