use editor_common::Tri;
use editor_crdt::Dot;
use editor_model::{ChildView, DocView, Modifier, ModifierType};
use editor_resource::{Resource, find_bold_target, find_unbold_target};
use editor_state::ResolvedSelection;
use editor_state::{PendingModifier, PendingModifiers};
use editor_state::{resolve_modifier_state, resolve_modifier_state_in_range};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

fn span_dots(view: &DocView, rs: &ResolvedSelection) -> Option<(Dot, Dot)> {
    let from = rs.from();
    let to = rs.to();

    let from_child = view.node(from.node())?.child_at(from.offset())?;
    let first = match from_child {
        ChildView::Leaf(l) => l.dot(),
        ChildView::Block(b) => b.dot()?,
    };

    let to_off = to.offset().checked_sub(1)?;
    let to_child = view.node(to.node())?.child_at(to_off)?;
    let last = match to_child {
        ChildView::Leaf(l) => l.dot(),
        ChildView::Block(b) => b.dot()?,
    };

    Some((first, last))
}

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

pub fn toggle_bold(tr: &mut Transaction, resource: &Resource) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    if selection.anchor == selection.head {
        return toggle_bold_collapsed(tr, resource);
    }

    let (first, last, current_weight, font_family, inherited_weight, is_bold) = {
        let view = tr.view();
        let rs = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let Some((first, last)) = span_dots(&view, &rs) else {
            return Ok(false);
        };
        let ms = resolve_modifier_state_in_range(&rs);
        let from_block = rs.from().node();
        let inherited_weight = block_weight(&view, from_block).ok_or_else(|| {
            CommandError::Corrupted("FontWeight missing in inherited modifiers".into())
        })?;
        let current_weight = match &ms.font_weight {
            Tri::Uniform { value } => value.value,
            _ => inherited_weight,
        };
        let font_family = match &ms.font_family {
            Tri::Uniform { value } => value.value.clone(),
            _ => block_family(&view, from_block).ok_or_else(|| {
                CommandError::Corrupted("FontFamily missing in effective modifiers".into())
            })?,
        };
        let is_bold = matches!(ms.effective_bold, Tri::Uniform { .. });
        (
            first,
            last,
            current_weight,
            font_family,
            inherited_weight,
            is_bold,
        )
    };

    let available = resource.font_registry.weights(&font_family).unwrap_or(&[]);

    if is_bold {
        tr.remove_span_modifier(first, last, Modifier::Bold)?;
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
        match find_bold_target(current_weight, available) {
            Some(target) => {
                tr.remove_span_modifier(
                    first,
                    last,
                    Modifier::FontWeight {
                        value: current_weight,
                    },
                )?;
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

    let (current_weight, font_family, is_bold) = {
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
        )
    };

    let inherited_weight = {
        let view = tr.view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
        match node.effective().get(&ModifierType::FontWeight) {
            Some(Modifier::FontWeight { value }) => *value,
            _ => {
                return Err(CommandError::Corrupted(
                    "FontWeight missing in inherited modifiers".into(),
                ));
            }
        }
    };

    let available = resource.font_registry.weights(&font_family).unwrap_or(&[]);

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
        assert!(actual.pending_modifiers.iter().any(|pm| matches!(
            pm,
            PendingModifier::Unset {
                ty: ModifierType::Bold
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
}
