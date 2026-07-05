use editor_common::Tri;
use editor_crdt::Dot;
use editor_model::{DEFAULT_FONT_FAMILY, DEFAULT_FONT_WEIGHT, DocView, Modifier, ModifierType};
use editor_resource::{Resource, find_bold_target, find_unbold_target};
use editor_state::{PendingModifier, PendingModifiers, leaf_span_in_range};
use editor_state::{ResolvedSelection, leaf_groups_in_range};
use editor_state::{resolve_modifier_state, resolve_modifier_state_in_range};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

fn block_weight(view: &DocView, elem: Dot) -> Option<u16> {
    match view.node(elem)?.effective().get(&ModifierType::FontWeight) {
        Some(Modifier::FontWeight { value }) => Some(*value),
        _ => None,
    }
}

fn block_family(view: &DocView, elem: Dot) -> Option<String> {
    match view.node(elem)?.effective().get(&ModifierType::FontFamily) {
        Some(Modifier::FontFamily { value }) => Some(value.clone()),
        _ => None,
    }
}

fn range_has_heavy_weight(rs: &ResolvedSelection) -> bool {
    leaf_groups_in_range(rs).iter().any(|g| {
        matches!(
            g.effective.get(&ModifierType::FontWeight),
            Some(Modifier::FontWeight { value }) if *value >= 700
        )
    })
}

pub fn toggle_bold(tr: &mut Transaction, resource: &Resource) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    if selection.anchor == selection.head {
        return toggle_bold_collapsed(tr, resource);
    }

    let (
        first,
        last,
        current_weight,
        font_family,
        inherited_weight,
        is_bold,
        synthetic_bold,
        weight_bold,
    ) = {
        let view = tr.view();
        let rs = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let Some((first, last)) = leaf_span_in_range(&rs) else {
            return Ok(false);
        };
        let ms = resolve_modifier_state_in_range(&rs);
        let from_block = rs.from().node();
        let inherited_weight = block_weight(&view, from_block).unwrap_or(DEFAULT_FONT_WEIGHT);
        let current_weight = match &ms.font_weight {
            Tri::Uniform { value } => value.value,
            _ => inherited_weight,
        };
        let font_family = match &ms.font_family {
            Tri::Uniform { value } => value.value.clone(),
            _ => block_family(&view, from_block).unwrap_or_else(|| DEFAULT_FONT_FAMILY.to_string()),
        };
        let is_bold = matches!(ms.effective_bold, Tri::Uniform { .. });
        let synthetic_bold = !matches!(ms.bold, Tri::Absent);
        let weight_bold = range_has_heavy_weight(&rs);
        (
            first,
            last,
            current_weight,
            font_family,
            inherited_weight,
            is_bold,
            synthetic_bold,
            weight_bold,
        )
    };

    let available = resource.font_registry.weights(&font_family).unwrap_or(&[]);

    if is_bold {
        if synthetic_bold {
            tr.remove_span_modifier(first, last, Modifier::Bold)?;
        }
        if weight_bold {
            let unbold = find_unbold_target(current_weight, available);
            tr.remove_span_modifier(
                first,
                last,
                Modifier::FontWeight {
                    value: current_weight,
                },
            )?;
            if unbold != inherited_weight {
                tr.add_span_modifier(first, last, Modifier::FontWeight { value: unbold })?;
            }
        } else {
            tr.remove_span_modifier(
                first,
                last,
                Modifier::FontWeight {
                    value: current_weight,
                },
            )?;
        }
    } else {
        match find_bold_target(current_weight, available) {
            Some(target) => {
                // The cancel-to-absent pass only matters when some FontWeight
                // span actually overlaps the range; on a pristine range it is a
                // whole-range no-op, and skipping it halves the styling cost.
                if tr
                    .state()
                    .projected
                    .span_of_type_overlaps(first, last, ModifierType::FontWeight)
                {
                    tr.remove_span_modifier(
                        first,
                        last,
                        Modifier::FontWeight {
                            value: current_weight,
                        },
                    )?;
                }
                if target != inherited_weight {
                    tr.add_span_modifier(first, last, Modifier::FontWeight { value: target })?;
                }
            }
            None => {
                tr.add_span_modifier(first, last, Modifier::Bold)?;
            }
        }
    }

    Ok(true)
}

fn toggle_bold_collapsed(tr: &mut Transaction, resource: &Resource) -> CommandResult {
    let selection = tr.selection().expect("entry caller guaranteed selection");
    let pos = selection.head;

    let (current_weight, font_family, is_bold, synthetic_bold, weight_bold) = {
        let ms = resolve_modifier_state(&tr.state().projected, &selection, tr.pending_modifiers())
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let current_weight = match &ms.font_weight {
            Tri::Uniform { value } => value.value,
            _ => {
                return Err(CommandError::Corrupted(
                    "FontWeight missing in effective modifiers".into(),
                ));
            }
        };
        let font_family = match &ms.font_family {
            Tri::Uniform { value } => value.value.clone(),
            _ => {
                return Err(CommandError::Corrupted(
                    "FontFamily missing in effective modifiers".into(),
                ));
            }
        };
        (
            current_weight,
            font_family,
            matches!(ms.effective_bold, Tri::Uniform { .. }),
            matches!(ms.bold, Tri::Uniform { .. }),
            current_weight >= 700,
        )
    };

    let inherited_weight = {
        let view = tr.view();
        view.node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
        block_weight(&view, pos.node).unwrap_or(DEFAULT_FONT_WEIGHT)
    };

    let available = resource.font_registry.weights(&font_family).unwrap_or(&[]);

    let mut replaced_types = Vec::new();
    if is_bold {
        if synthetic_bold {
            replaced_types.push(ModifierType::Bold);
        }
        replaced_types.push(ModifierType::FontWeight);
    } else if find_bold_target(current_weight, available).is_some() {
        replaced_types.push(ModifierType::FontWeight);
    } else {
        replaced_types.push(ModifierType::Bold);
    }

    let mut pending: PendingModifiers = tr
        .pending_modifiers()
        .iter()
        .filter(|pm| !replaced_types.contains(&pm.as_type()))
        .cloned()
        .collect();

    if is_bold {
        if synthetic_bold {
            pending.push(PendingModifier::Unset {
                ty: ModifierType::Bold,
            });
        }
        let unbold = if weight_bold {
            find_unbold_target(current_weight, available)
        } else {
            DEFAULT_FONT_WEIGHT
        };
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
    fn collapsed_toggle_on_sets_pending_font_weight() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 3)
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
                    p1: paragraph {
                        text("Hello")
                    }
                }
            }
            selection: (p1, 3)
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
        let resource = make_resource([("Pretendard", vec![400])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 3)
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
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [bold, font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        assert!(actual.pending_modifiers.iter().any(|pm| matches!(
            pm,
            PendingModifier::Unset {
                ty: ModifierType::Bold
            }
        )));
        assert!(!actual.pending_modifiers.iter().any(|pm| matches!(
            pm,
            PendingModifier::Set {
                modifier: Modifier::FontWeight { .. }
            }
        )));
        assert!(actual.pending_modifiers.iter().any(|pm| matches!(
            pm,
            PendingModifier::Unset {
                ty: ModifierType::FontWeight
            }
        )));
    }

    #[test]
    fn collapsed_toggle_off_from_heavy_weight() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        assert!(
            !actual
                .pending_modifiers
                .iter()
                .any(|pm| pm.as_type() == ModifierType::Bold)
        );
        assert!(actual.pending_modifiers.iter().any(|pm| matches!(
            pm,
            PendingModifier::Unset {
                ty: ModifierType::FontWeight
            }
        )));
    }

    #[test]
    fn collapsed_can_toggle_back_on_after_unbolding_to_inherited_weight() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| {
            toggle_bold(&mut tr, &resource).unwrap();
            toggle_bold(&mut tr, &resource).unwrap();
            toggle_bold(&mut tr, &resource)
        });
        assert_eq!(
            actual.pending_modifiers.as_slice(),
            &[PendingModifier::Set {
                modifier: Modifier::FontWeight { value: 700 }
            }]
        );
    }

    #[test]
    fn collapsed_toggle_off_unbold_differs_from_inherited() {
        let resource = make_resource([("Pretendard", vec![300, 400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(300), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 3)
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
                    p1: paragraph {
                        text("Hello") [font_weight(300), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        assert!(!actual.pending_modifiers.iter().any(|pm| matches!(
            pm,
            PendingModifier::Set {
                modifier: Modifier::FontWeight { .. }
            }
        )));
    }

    // On a document whose range carries no FontWeight span at all, the
    // cancel-to-absent RemoveSpan is a provable no-op — bolding must emit only
    // the AddSpan, not a second whole-range op.
    #[test]
    fn range_toggle_on_pristine_emits_single_span_op() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut tr = Transaction::new(&initial);
        assert!(toggle_bold(&mut tr, &resource).unwrap());
        let (_state, _steps, recorded, ..) = tr.commit();
        let span_ops = recorded
            .iter()
            .filter(|r| matches!(r.op.payload, editor_model::EditOp::Span(_)))
            .count();
        assert_eq!(
            span_ops, 1,
            "pristine bold-on must emit only AddSpan(FontWeight)"
        );
    }

    // With an explicit FontWeight span in range, the cancel must still be
    // emitted — and a full toggle round-trip must restore the original state.
    #[test]
    fn range_toggle_on_with_existing_weight_span_still_cancels() {
        let resource = make_resource([("Pretendard", vec![300, 400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph { text("Hello") [font_weight(300)] }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph { text("Hello") [font_weight(700)] }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_on_single_node_full() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn block_unit_selection_toggle_on_applies_weight_to_text() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                r1: root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                r1: root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_on_partial_applies_weight_to_substring() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p: paragraph {
                        text("Hello World") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p, 6) -> (p, 11)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p: paragraph {
                        text("Hello ") [font_weight(400), font_family("Pretendard".to_string())]
                        text("World") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p, 6) -> (p, 11)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_on_makes_mixed_range_uniform_bold() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p: paragraph {
                        text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                        text("Bold") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p, 0) -> (p, 9)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p: paragraph {
                        text("HelloBold") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p, 0) -> (p, 9)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_on_removes_redundant_weight() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(700), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(700), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    // SC-FMT-3: toggling bold off leaves no effective Bold on any covered leaf,
    // and that absence is achieved purely by removing the record (Task 1 made
    // Bold non-inheritable, so removal and blocking are observationally identical).
    #[test]
    fn range_toggle_off_effective_bold_absent_by_record_removal() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("가나") [bold, font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let view = actual.view();
        let para = view.roots().next().unwrap().child_blocks().next().unwrap();
        let count = para.children().count();
        assert!(count > 0, "paragraph must have char leaves");
        for i in 0..count {
            let st = para.leaf_state_at(i).expect("char leaf has state");
            assert!(
                st.eff.get(&ModifierType::Bold).is_none(),
                "toggle-off leaves no effective Bold at leaf {i}"
            );
        }
    }

    #[test]
    fn range_toggle_off_removes_bold_and_resets_weight() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [bold, font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_off_heavy_weight_no_bold_marker() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_off_inherited_synthetic_bold_reports_default_weight() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [bold]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        assert_eq!(
            resolve_modifier_state(
                &actual.projected,
                actual.selection.as_ref().unwrap(),
                &actual.pending_modifiers
            )
            .unwrap()
            .font_weight,
            Tri::Uniform {
                value: editor_model::FontWeightValue { value: 400 }
            }
        );
    }

    #[test]
    fn range_toggle_off_synthetic_bold_resets_mixed_nonbold_weights() {
        let resource = make_resource([("Pretendard", vec![300, 400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("A") [bold, font_weight(300)]
                        text("B") [bold, font_weight(400)]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let ms = resolve_modifier_state(
            &actual.projected,
            actual.selection.as_ref().unwrap(),
            &actual.pending_modifiers,
        )
        .unwrap();
        assert_eq!(ms.effective_bold, Tri::Absent);
        assert_eq!(
            ms.font_weight,
            Tri::Uniform {
                value: editor_model::FontWeightValue { value: 400 }
            }
        );
    }

    #[test]
    fn range_toggle_off_keeps_nondefault_unbold_weight() {
        let resource = make_resource([("Pretendard", vec![300, 400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(300), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(300), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_off_multi_run() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p: paragraph {
                        text("Hello") [bold, font_weight(700), font_family("Pretendard".to_string())]
                        text("World") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p, 0) -> (p, 10)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p: paragraph {
                        text("HelloWorld") [font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p, 0) -> (p, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_on_cross_paragraph() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                    p2: paragraph {
                        text("World") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                    p2: paragraph {
                        text("World") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p2, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_on_faux_bold_when_no_heavier() {
        let resource = make_resource([("Pretendard", vec![400])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [bold, font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_double_toggle_restores_original() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello World") [font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 6) -> (p1, 11)
        };
        let (actual, ..) = transact!(initial, |tr| {
            toggle_bold(&mut tr, &resource).unwrap();
            toggle_bold(&mut tr, &resource)
        });
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello World") [font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 6) -> (p1, 11)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_double_toggle_reports_rendered_weight() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello World")
                    }
                }
            }
            selection: (p1, 6) -> (p1, 11)
        };
        let (actual, ..) = transact!(initial, |tr| {
            toggle_bold(&mut tr, &resource).unwrap();
            toggle_bold(&mut tr, &resource)
        });

        let ms = resolve_modifier_state(
            &actual.projected,
            actual.selection.as_ref().unwrap(),
            &actual.pending_modifiers,
        )
        .unwrap();
        assert_eq!(
            ms.font_weight,
            Tri::Uniform {
                value: editor_model::FontWeightValue { value: 400 }
            }
        );
    }

    #[test]
    fn backward_selection_applies_weight() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(400), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 5) -> (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_bold(&mut tr, &resource));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("Hello") [font_weight(700), font_family("Pretendard".to_string())]
                    }
                }
            }
            selection: (p1, 5) -> (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn block_level_selection_does_not_corrupt() {
        let resource = make_resource([("Pretendard", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                r1: root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        text("hello")
                    }
                    paragraph {
                        text("world")
                    }
                }
            }
            selection: (r1, 0, >) -> (r1, 2, <)
        };
        let mut tr = editor_transaction::Transaction::new(&initial);
        let result = toggle_bold(&mut tr, &resource);
        assert!(
            matches!(result, Ok(true)),
            "block-level selection over a root carrying its weight via a style must not report corruption, got {result:?}"
        );
    }
}
