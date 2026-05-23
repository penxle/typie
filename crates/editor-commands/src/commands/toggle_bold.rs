use editor_model::{Modifier, ModifierType, NodeId, NodeRef};
use editor_resource::{Resource, match_weight};
use editor_state::{PendingModifier, PendingModifiers, Position, resolve_effective_modifiers_at};
use editor_transaction::Transaction;

use crate::helpers::{
    collect_text_nodes_in_range, compact_and_restore_selection, filter_applicable_node_ids,
    resolve_inherited_modifiers,
};
use crate::{CommandError, CommandResult};

pub fn toggle_bold(tr: &mut Transaction, resource: &Resource) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    if selection.is_collapsed() {
        return toggle_bold_collapsed(tr, resource);
    }

    let doc = tr.doc();
    let resolved = selection
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
    let from = Position::from(resolved.from());
    let to = Position::from(resolved.to());

    let node_ids = collect_text_nodes_in_range(tr, &from, &to)?;
    let applicable_node_ids = filter_applicable_node_ids(&tr.doc(), &node_ids, ModifierType::Bold);

    if applicable_node_ids.is_empty() {
        return Ok(false);
    }

    let doc = tr.doc();
    let nodes = applicable_node_ids
        .iter()
        .filter_map(|id| doc.node(*id))
        .collect::<Vec<_>>();
    let is_bold = check_range_is_bold(&nodes);

    if is_bold {
        toggle_bold_off_range(tr, resource, &applicable_node_ids)?;
    } else {
        toggle_bold_on_range(tr, resource, &applicable_node_ids)?;
    }

    compact_and_restore_selection(tr, &node_ids)?;

    Ok(true)
}

/// Selects bold target weight from available weights.
/// Returns None if no heavier weight exists (use faux bold).
fn find_bold_target(current_weight: u16, available_weights: &[u16]) -> Option<u16> {
    let candidates: Vec<u16> = available_weights
        .iter()
        .copied()
        .filter(|&w| w > current_weight)
        .collect();

    if candidates.is_empty() {
        return None;
    }

    let bold_candidates: Vec<u16> = candidates.iter().copied().filter(|&w| w >= 700).collect();

    let pool = if bold_candidates.is_empty() {
        &candidates
    } else {
        &bold_candidates
    };

    nearest_in(pool, 700)
}

/// Selects unbold target weight from available weights.
/// Falls back to 400 if no lighter weight exists.
fn find_unbold_target(current_weight: u16, available_weights: &[u16]) -> u16 {
    let candidates: Vec<u16> = available_weights
        .iter()
        .copied()
        .filter(|&w| w < current_weight)
        .collect();

    if candidates.is_empty() {
        return 400;
    }

    nearest_in(&candidates, 400).unwrap_or(400)
}

/// Pick the weight nearest to `target` using CSS Fonts Level 4 section 5.2 matching.
fn nearest_in(weights: &[u16], target: u16) -> Option<u16> {
    let mut sorted = weights.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    match_weight(&sorted, target)
}

fn is_node_bold(node: &NodeRef) -> bool {
    if node.modifiers().any(|m| matches!(m, Modifier::Bold)) {
        return true;
    }
    let weight = node
        .modifiers()
        .find_map(|m| match m {
            Modifier::FontWeight { value } => Some(*value),
            _ => None,
        })
        .unwrap_or_else(|| {
            resolve_inherited_modifiers(node)
                .iter()
                .find_map(|m| match m {
                    Modifier::FontWeight { value } => Some(*value),
                    _ => None,
                })
                .unwrap()
        });
    weight >= 700
}

fn check_range_is_bold(nodes: &[NodeRef]) -> bool {
    nodes.iter().all(|node| is_node_bold(node))
}

fn toggle_bold_on_range(
    tr: &mut Transaction,
    resource: &Resource,
    node_ids: &[NodeId],
) -> CommandResult {
    for &node_id in node_ids {
        let doc = tr.doc();
        let node = doc
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;

        if is_node_bold(&node) {
            continue;
        }

        let inherited = resolve_inherited_modifiers(&node);
        let inherited_weight = inherited
            .iter()
            .find_map(|m| match m {
                Modifier::FontWeight { value } => Some(*value),
                _ => None,
            })
            .unwrap();
        let inherited_family = inherited
            .iter()
            .find_map(|m| match m {
                Modifier::FontFamily { value } => Some(value.as_str()),
                _ => None,
            })
            .unwrap();

        let has_explicit_weight = node.modifiers().find_map(|m| match m {
            Modifier::FontWeight { value } => Some(*value),
            _ => None,
        });
        let current_weight = has_explicit_weight.unwrap_or(inherited_weight);
        let font_family = node
            .modifiers()
            .find_map(|m| match m {
                Modifier::FontFamily { value } => Some(value.as_str()),
                _ => None,
            })
            .unwrap_or(inherited_family);

        let available = resource.font_registry.weights(font_family).unwrap_or(&[]);

        match find_bold_target(current_weight, available) {
            Some(target) => {
                if let Some(w) = has_explicit_weight {
                    tr.remove_modifier(node_id, Modifier::FontWeight { value: w })?;
                }
                if target != inherited_weight {
                    tr.add_modifier(node_id, Modifier::FontWeight { value: target })?;
                }
            }
            None => {
                tr.add_modifier(node_id, Modifier::Bold)?;
            }
        }
    }

    Ok(true)
}

fn toggle_bold_off_range(
    tr: &mut Transaction,
    resource: &Resource,
    node_ids: &[NodeId],
) -> CommandResult {
    for &node_id in node_ids {
        let doc = tr.doc();
        let node = doc
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;

        let has_bold = node.modifiers().any(|m| matches!(m, Modifier::Bold));
        let inherited = resolve_inherited_modifiers(&node);
        let inherited_weight = inherited
            .iter()
            .find_map(|m| match m {
                Modifier::FontWeight { value } => Some(*value),
                _ => None,
            })
            .unwrap();
        let inherited_family = inherited
            .iter()
            .find_map(|m| match m {
                Modifier::FontFamily { value } => Some(value.as_str()),
                _ => None,
            })
            .unwrap();

        let has_explicit_weight = node.modifiers().find_map(|m| match m {
            Modifier::FontWeight { value } => Some(*value),
            _ => None,
        });
        let current_weight = has_explicit_weight.unwrap_or(inherited_weight);
        let font_family = node
            .modifiers()
            .find_map(|m| match m {
                Modifier::FontFamily { value } => Some(value.as_str()),
                _ => None,
            })
            .unwrap_or(inherited_family);

        let available = resource.font_registry.weights(font_family).unwrap_or(&[]);
        let unbold = find_unbold_target(current_weight, available);

        if has_bold {
            tr.remove_modifier(node_id, Modifier::Bold)?;
        }

        if let Some(w) = has_explicit_weight {
            tr.remove_modifier(node_id, Modifier::FontWeight { value: w })?;
        }
        if unbold != inherited_weight {
            tr.add_modifier(node_id, Modifier::FontWeight { value: unbold })?;
        }
    }

    Ok(true)
}

fn toggle_bold_collapsed(tr: &mut Transaction, resource: &Resource) -> CommandResult {
    let pos = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let effective = resolve_effective_modifiers_at(tr.state(), &pos);

    let has_bold = effective.iter().any(|m| matches!(m, Modifier::Bold));
    let current_weight = effective
        .iter()
        .find_map(|m| match m {
            Modifier::FontWeight { value } => Some(*value),
            _ => None,
        })
        .ok_or_else(|| {
            CommandError::Corrupted("FontWeight missing in effective modifiers".into())
        })?;
    let font_family = effective
        .iter()
        .find_map(|m| match m {
            Modifier::FontFamily { value } => Some(value.as_str()),
            _ => None,
        })
        .ok_or_else(|| {
            CommandError::Corrupted("FontFamily missing in effective modifiers".into())
        })?;

    let available = resource.font_registry.weights(font_family).unwrap_or(&[]);
    let inherited = resolve_inherited_modifiers(&node);
    let inherited_weight = inherited
        .iter()
        .find_map(|m| match m {
            Modifier::FontWeight { value } => Some(*value),
            _ => None,
        })
        .ok_or_else(|| {
            CommandError::Corrupted("FontWeight missing in inherited modifiers".into())
        })?;
    let is_bold = has_bold || current_weight >= 700;

    let mut pending: PendingModifiers = tr
        .pending_modifiers()
        .iter()
        .filter(|pm| {
            let t = pm.as_type();
            t != ModifierType::Bold && t != ModifierType::FontWeight
        })
        .cloned()
        .collect();

    if is_bold {
        let unbold = find_unbold_target(current_weight, available);
        pending.push(PendingModifier::Unset {
            ty: ModifierType::Bold,
        });
        if unbold != inherited_weight {
            pending.push(PendingModifier::Set {
                modifier: Modifier::FontWeight { value: unbold },
            });
        } else {
            pending.push(PendingModifier::Unset {
                ty: ModifierType::FontWeight,
            });
        }
    } else {
        match find_bold_target(current_weight, available) {
            Some(target) => {
                if target != inherited_weight {
                    pending.push(PendingModifier::Set {
                        modifier: Modifier::FontWeight { value: target },
                    });
                } else {
                    pending.push(PendingModifier::Unset {
                        ty: ModifierType::FontWeight,
                    });
                }
            }
            None => {
                pending.push(PendingModifier::Set {
                    modifier: Modifier::Bold,
                });
            }
        }
    }

    tr.set_pending_modifiers(pending)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_resource::{FontFamily, FontFamilySource, FontWeight};

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn toggle_bold_returns_false_when_no_selection() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc { root { paragraph { text("Hello") [font_weight(400), font_family("Pretendard".to_string())] } } }
            selection: none
        };
        let mut tr = editor_transaction::Transaction::new(&initial);
        let result = toggle_bold(&mut tr, &resource);
        assert!(matches!(result, Ok(false)));
    }

    #[test]
    fn bold_target_prefers_700_when_available() {
        assert_eq!(
            find_bold_target(400, &[100, 300, 400, 500, 700, 900]),
            Some(700)
        );
    }

    #[test]
    fn bold_target_picks_nearest_bold_candidate() {
        assert_eq!(find_bold_target(400, &[400, 800, 900]), Some(800));
    }

    #[test]
    fn bold_target_uses_heavier_even_below_700() {
        assert_eq!(find_bold_target(400, &[400, 500]), Some(500));
    }

    #[test]
    fn bold_target_none_when_no_heavier() {
        assert_eq!(find_bold_target(900, &[400, 700, 900]), None);
    }

    #[test]
    fn bold_target_none_when_already_heaviest() {
        assert_eq!(find_bold_target(400, &[400]), None);
    }

    #[test]
    fn bold_target_from_300() {
        assert_eq!(find_bold_target(300, &[100, 300, 400, 700]), Some(700));
    }

    #[test]
    fn unbold_target_prefers_400() {
        assert_eq!(
            find_unbold_target(700, &[100, 300, 400, 500, 700, 900]),
            400
        );
    }

    #[test]
    fn unbold_target_picks_nearest_to_400() {
        assert_eq!(find_unbold_target(700, &[100, 300, 700]), 300);
    }

    #[test]
    fn unbold_target_defaults_to_400_when_no_lighter() {
        assert_eq!(find_unbold_target(100, &[100, 700]), 400);
    }

    #[test]
    fn unbold_target_from_900() {
        assert_eq!(find_unbold_target(900, &[400, 700, 900]), 400);
    }

    fn make_resource(families: impl IntoIterator<Item = (&'static str, Vec<u16>)>) -> Resource {
        let mut resource = Resource::new_test();
        let families: Vec<FontFamily> = families
            .into_iter()
            .map(|(name, weights)| FontFamily {
                name: name.into(),
                source: FontFamilySource::Default,
                weights: weights
                    .into_iter()
                    .map(|w| FontWeight {
                        value: w,
                        hash: format!("h_{}_{}", name, w),
                        chunks: vec![vec![0x0000, 0xFFFF]],
                    })
                    .collect(),
            })
            .collect();
        resource.font_registry.set_fonts(families);
        resource
    }

    #[test]
    fn collapsed_toggle_on_sets_pending_font_weight() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        assert_eq!(
            actual.pending_modifiers.as_slice(),
            &[PendingModifier::Set {
                modifier: Modifier::FontWeight { value: 700 }
            }]
        );
    }

    #[test]
    fn collapsed_toggle_on_uses_inherited_font_weight() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        assert_eq!(
            actual.pending_modifiers.as_slice(),
            &[PendingModifier::Set {
                modifier: Modifier::FontWeight { value: 700 }
            }]
        );
    }

    #[test]
    fn collapsed_toggle_on_faux_bold_when_no_heavier() {
        // Weight 400, only available weight is [400] → no heavier → faux bold
        let resource = make_resource([("Pretendard", vec![400])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        assert_eq!(
            actual.pending_modifiers.as_slice(),
            &[PendingModifier::Set {
                modifier: Modifier::Bold
            }]
        );
    }

    #[test]
    fn collapsed_toggle_off_from_bold_modifier() {
        // Text has Bold modifier, weight 400 → toggle off removes Bold, weight stays inherited
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [bold, font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        assert!(actual.pending_modifiers.iter().any(|pm| matches!(
            pm,
            PendingModifier::Unset {
                ty: ModifierType::Bold
            }
        )));
        // unbold target 400 == inherited 400 → no FontWeight set
        assert!(!actual.pending_modifiers.iter().any(|pm| matches!(
            pm,
            PendingModifier::Set {
                modifier: Modifier::FontWeight { .. }
            }
        )));
    }

    #[test]
    fn collapsed_toggle_off_from_heavy_weight() {
        // Weight 700 (no Bold marker) → toggle off to 400
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        assert!(actual.pending_modifiers.iter().any(|pm| matches!(
            pm,
            PendingModifier::Unset {
                ty: ModifierType::Bold
            }
        )));
        // unbold target 400 == inherited 400 → FontWeight unset (not set)
        assert!(actual.pending_modifiers.iter().any(|pm| matches!(
            pm,
            PendingModifier::Unset {
                ty: ModifierType::FontWeight
            }
        )));
    }

    #[test]
    fn collapsed_toggle_off_unbold_differs_from_inherited() {
        // Inherited 300, current 700, unbold target picks 400 → 400 != 300 → set FontWeight(400)
        let resource = make_resource([("Pretendard", vec![300, 400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(300), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        assert!(actual.pending_modifiers.iter().any(|pm| matches!(
            pm,
            PendingModifier::Set {
                modifier: Modifier::FontWeight { value: 400 }
            }
        )));
    }

    #[test]
    fn collapsed_toggle_on_no_redundant_weight() {
        let resource = make_resource([("Pretendard", vec![300, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(700), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(300), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        assert!(!actual.pending_modifiers.iter().any(|pm| matches!(
            pm,
            PendingModifier::Set {
                modifier: Modifier::FontWeight { .. }
            }
        )));
    }

    #[test]
    fn check_bold_all_bold_nodes() {
        let (state, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [bold, font_weight(400), font_family("Pretendard".to_string())]
                        t2: text("World") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        let doc = state.doc;
        let sel = state.selection.as_ref().unwrap();
        let nodes = vec![
            doc.node(sel.anchor.node_id).unwrap(),
            doc.node(sel.head.node_id).unwrap(),
        ];
        assert!(check_range_is_bold(&nodes));
    }

    #[test]
    fn check_bold_mixed_returns_false() {
        let (state, t1, t2) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [bold, font_weight(400), font_family("Pretendard".to_string())]
                        t2: text("World") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        let doc = state.doc;
        let nodes = vec![doc.node(t1).unwrap(), doc.node(t2).unwrap()];
        assert!(!check_range_is_bold(&nodes));
    }

    #[test]
    fn range_toggle_on_single_node_full() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_on_partial_splits_node() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello World") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 6) -> (t1, 11)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello ") [font_weight(400), font_family("Pretendard".to_string())]
                        t2: text("World") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t2, 0) -> (t2, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_on_skips_already_bold() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                        t2: text("Bold") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 4)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("HelloBold") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 9)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_bold_on_fold_title_only_is_noop() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    fold {
                        fold_title { t1: text("Title") }
                        fold_content { paragraph { text("Body") } }
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact_fail!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    fold {
                        fold_title { t1: text("Title") }
                        fold_content { paragraph { text("Body") } }
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_bold_skips_fold_title_applies_to_paragraph() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    fold {
                        fold_title { t1: text("Title") }
                        fold_content { paragraph { t2: text("Body") } }
                    }
                }
            }
            selection: (t1, 0) -> (t2, 4)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    fold {
                        fold_title { t1: text("Title") }
                        fold_content { paragraph { t2: text("Body") [font_weight(700)] } }
                    }
                }
            }
            selection: (t1, 0) -> (t2, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_on_removes_redundant_weight() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(700), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let t1_id = actual.selection.as_ref().unwrap().head.node_id;
        let entry = actual.doc.get_entry(t1_id).unwrap();
        assert!(
            !entry
                .modifiers
                .iter()
                .any(|(_, m)| matches!(m, Modifier::FontWeight { .. }))
        );
    }

    #[test]
    fn range_toggle_off_removes_bold_and_sets_weight() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [bold, font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let t1_id = actual.selection.as_ref().unwrap().head.node_id;
        let entry = actual.doc.get_entry(t1_id).unwrap();
        assert!(
            !entry
                .modifiers
                .iter()
                .any(|(_, m)| matches!(m, Modifier::Bold))
        );
        assert!(
            !entry
                .modifiers
                .iter()
                .any(|(_, m)| matches!(m, Modifier::FontWeight { .. }))
        );
    }

    #[test]
    fn range_toggle_off_heavy_weight_no_bold_marker() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let t1_id = actual.selection.as_ref().unwrap().head.node_id;
        let entry = actual.doc.get_entry(t1_id).unwrap();
        assert!(
            !entry
                .modifiers
                .iter()
                .any(|(_, m)| matches!(m, Modifier::FontWeight { .. }))
        );
    }

    #[test]
    fn range_toggle_off_keeps_nondefault_unbold_weight() {
        let resource = make_resource([("Pretendard", vec![300, 400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(300), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let t1_id = actual.selection.as_ref().unwrap().head.node_id;
        let entry = actual.doc.get_entry(t1_id).unwrap();
        assert!(
            entry
                .modifiers
                .iter()
                .any(|(_, m)| matches!(m, Modifier::FontWeight { value: 400 }))
        );
    }

    #[test]
    fn range_toggle_off_multi_node() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [bold, font_weight(700), font_family("Pretendard".to_string())]
                        t2: text("World") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("HelloWorld") [font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_on_cross_paragraph() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                    paragraph {
                        t2: text("World") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                    paragraph {
                        t2: text("World") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn backward_selection_works() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 5) -> (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let t1_id = actual.selection.as_ref().unwrap().anchor.node_id;
        let entry = actual.doc.get_entry(t1_id).unwrap();
        assert!(
            entry
                .modifiers
                .iter()
                .any(|(_, m)| matches!(m, Modifier::FontWeight { value: 700 }))
        );
    }

    #[test]
    fn compact_merges_adjacent_after_toggle_on() {
        // Two nodes: normal + already bold → toggle ON makes first bold too → compact merges
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                        t2: text("World") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("HelloWorld") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn compact_merges_adjacent_after_toggle_off() {
        // Two bold nodes → toggle OFF → both become normal → compact merges
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                        t2: text("World") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("HelloWorld") [font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn compact_merges_split_node_back_after_full_bold() {
        // Partial selection splits node, if result has same modifiers as neighbor → compact merges
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("AB") [font_weight(400), font_family("Pretendard".to_string())]
                        t2: text("CD") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 1) -> (t2, 2)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        // "A" stays 400, "B" becomes 700, "CD" stays 700 → "B"+"CD" merge
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("A") [font_weight(400), font_family("Pretendard".to_string())]
                        t2: text("BCD") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t2, 0) -> (t2, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn no_compact_when_modifiers_differ() {
        // Bold + non-bold stay separate after toggle
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello World") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        // "Hello" becomes 700, " World" stays 400 → no merge (different modifiers)
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                        t2: text(" World") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_double_toggle_preserves_valid_selection() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello World") [font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 6) -> (t1, 11)
        };

        let (actual, ..) = transact!(initial, |tr| {
            toggle_bold(&mut tr, &resource).unwrap();
            toggle_bold(&mut tr, &resource)
        });

        let sel = actual.selection.as_ref().unwrap();
        let doc = &actual.doc;
        assert!(
            doc.get_entry(sel.anchor.node_id).is_some(),
            "anchor node must exist"
        );
        assert!(
            doc.get_entry(sel.head.node_id).is_some(),
            "head node must exist"
        );
    }

    #[test]
    fn range_double_toggle_with_adjacent_hard_break() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_family("Pretendard".to_string())]
                        hard_break
                        t2: text("World") [font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };

        let (actual, ..) = transact!(initial, |tr| {
            toggle_bold(&mut tr, &resource).unwrap();
            toggle_bold(&mut tr, &resource)
        });

        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_family("Pretendard".to_string())]
                        hard_break
                        t2: text("World") [font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_double_toggle_across_hard_break() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_family("Pretendard".to_string())]
                        hard_break
                        t2: text("World") [font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };

        let (actual, ..) = transact!(initial, |tr| {
            toggle_bold(&mut tr, &resource).unwrap();
            toggle_bold(&mut tr, &resource)
        });

        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_family("Pretendard".to_string())]
                        hard_break
                        t2: text("World") [font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_on_faux_bold_when_no_heavier() {
        // Weight 400, only [400] available → no heavier → faux bold
        let resource = make_resource([("Pretendard", vec![400])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello") [bold, font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }
}
