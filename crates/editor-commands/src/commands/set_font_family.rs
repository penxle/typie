use std::collections::BTreeMap;

use editor_common::Tri;
use editor_crdt::Dot;
use editor_model::{ChildView, DEFAULT_FONT_WEIGHT, DocView, Modifier, ModifierType, NodeView};
use editor_resource::{Resource, find_bold_target, match_weight};
use editor_state::{
    PendingModifier, PendingModifiers, ResolvedSelection, inline_leaf_dots_in_range,
    resolve_modifier_state,
};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn set_font_family(tr: &mut Transaction, value: String, resource: &Resource) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let Some(weights) = resource
        .font_registry
        .weights(&value)
        .filter(|w| !w.is_empty())
    else {
        return Ok(false);
    };

    let family = Modifier::FontFamily { value };
    if selection.anchor == selection.head {
        set_collapsed(tr, family, weights)
    } else {
        set_range(tr, family, weights)
    }
}

fn font_weight(effective: &BTreeMap<ModifierType, Modifier>) -> u16 {
    match effective.get(&ModifierType::FontWeight) {
        Some(Modifier::FontWeight { value }) => *value,
        _ => DEFAULT_FONT_WEIGHT,
    }
}

fn has_bold(effective: &BTreeMap<ModifierType, Modifier>) -> bool {
    effective.contains_key(&ModifierType::Bold)
}

fn weight_and_bold_after_family_change(
    old_weight: u16,
    old_bold: bool,
    available_weights: &[u16],
) -> (u16, bool) {
    let matched = match_weight(available_weights, old_weight).unwrap_or(old_weight);
    if old_bold {
        return find_bold_target(matched, available_weights)
            .map(|target| (target, false))
            .unwrap_or((matched, true));
    }
    if old_weight >= 700 && matched < 700 {
        return find_bold_target(matched, available_weights)
            .map(|target| (target, false))
            .unwrap_or((matched, true));
    }
    (matched, false)
}

fn provided_and_override(
    view: &DocView,
    block: Dot,
    offset: usize,
    ty: ModifierType,
) -> (Option<Modifier>, bool) {
    let Some(node) = view.node(block) else {
        return (None, false);
    };
    let block_eff = node.effective().get(&ty).cloned();
    let leaf_idx = offset.saturating_sub(1);
    let leaf = match node.child_at(leaf_idx) {
        Some(ChildView::Leaf(l)) => Some(l),
        _ => None,
    };
    let own = leaf.as_ref().and_then(|l| {
        l.own_modifiers()
            .get(&ty)
            .map(|o| (o.value.clone(), o.from_style))
    });
    let has_explicit_override = matches!(&own, Some((_, false)));
    let provided = match &own {
        Some((value, true)) => Some(value.clone()),
        Some((_, false)) => block_eff,
        None => leaf
            .as_ref()
            .and_then(|l| l.effective().get(&ty).cloned())
            .or(block_eff),
    };
    (provided, has_explicit_override)
}

fn set_collapsed(
    tr: &mut Transaction,
    family: Modifier,
    available_weights: &[u16],
) -> CommandResult {
    let selection = tr.selection().expect("entry caller guaranteed selection");
    let pos = selection.head;

    let (provided_family, explicit_family, old_weight, old_bold, inherited_weight) = {
        let ms = resolve_modifier_state(&tr.state().projected, &selection, tr.pending_modifiers())
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let view = tr.view();
        view.node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
        let current_weight = match &ms.font_weight {
            Tri::Uniform { value } => value.value,
            _ => DEFAULT_FONT_WEIGHT,
        };
        let inherited_weight = view
            .node(pos.node)
            .map(|node| font_weight(node.effective()))
            .unwrap_or(DEFAULT_FONT_WEIGHT);
        let (provided, explicit) =
            provided_and_override(&view, pos.node, pos.offset, ModifierType::FontFamily);
        (
            provided,
            explicit,
            current_weight,
            matches!(ms.bold, Tri::Uniform { .. }),
            inherited_weight,
        )
    };

    let (new_weight, new_bold) =
        weight_and_bold_after_family_change(old_weight, old_bold, available_weights);
    let changes_weight = new_weight != old_weight;
    let changes_bold = new_bold != old_bold;

    let mut pending: PendingModifiers = tr
        .pending_modifiers()
        .iter()
        .filter(|pm| {
            let ty = pm.as_type();
            ty != ModifierType::FontFamily
                && !(changes_weight && ty == ModifierType::FontWeight)
                && !(changes_bold && ty == ModifierType::Bold)
        })
        .cloned()
        .collect();

    if provided_family.as_ref() == Some(&family) {
        if explicit_family {
            pending.push(PendingModifier::Unset {
                ty: ModifierType::FontFamily,
            });
        }
    } else {
        pending.push(PendingModifier::Set { modifier: family });
    }

    if changes_bold {
        if new_bold {
            pending.push(PendingModifier::Set {
                modifier: Modifier::Bold,
            });
        } else {
            pending.push(PendingModifier::Unset {
                ty: ModifierType::Bold,
            });
        }
    }

    if changes_weight {
        if new_weight == inherited_weight {
            pending.push(PendingModifier::Unset {
                ty: ModifierType::FontWeight,
            });
        } else {
            pending.push(PendingModifier::Set {
                modifier: Modifier::FontWeight { value: new_weight },
            });
        }
    }

    tr.set_pending_modifiers(pending)?;
    Ok(true)
}

fn set_range(tr: &mut Transaction, family: Modifier, available_weights: &[u16]) -> CommandResult {
    let selection = tr.selection().expect("entry caller guaranteed selection");
    let (first, last, inherited_eq, leaves) = {
        let view = tr.view();
        let rs = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let Some((first, last)) = span_dots(&view, &rs) else {
            return Ok(false);
        };
        let from_block = rs.from().node();
        let inherited = view
            .node(from_block)
            .and_then(|n| n.effective().get(&ModifierType::FontFamily).cloned());
        let leaves = inline_leaf_dots_in_range(&view, &rs.from().position(), &rs.to().position())
            .into_iter()
            .filter_map(|dot| {
                let leaf = view.leaf(dot)?;
                Some((
                    dot,
                    font_weight(leaf.effective()),
                    leaf.parent()
                        .map(|node| font_weight(node.effective()))
                        .unwrap_or(DEFAULT_FONT_WEIGHT),
                    has_bold(leaf.effective()),
                ))
            })
            .collect::<Vec<_>>();
        (first, last, inherited.as_ref() == Some(&family), leaves)
    };

    if inherited_eq {
        tr.remove_span_modifier(first, last, family.clone())?;
    } else {
        tr.add_span_modifier(first, last, family.clone())?;
    }

    for (dot, old_weight, inherited_weight, old_bold) in leaves {
        let (new_weight, new_bold) =
            weight_and_bold_after_family_change(old_weight, old_bold, available_weights);

        if old_bold && !new_bold {
            tr.remove_span_modifier(dot, dot, Modifier::Bold)?;
        } else if !old_bold && new_bold {
            tr.add_span_modifier(dot, dot, Modifier::Bold)?;
        }

        if new_weight != old_weight {
            tr.remove_span_modifier(dot, dot, Modifier::FontWeight { value: old_weight })?;
            if new_weight != inherited_weight {
                tr.add_span_modifier(dot, dot, Modifier::FontWeight { value: new_weight })?;
            }
        }
    }

    Ok(true)
}

fn last_leaf_dot(block: &NodeView) -> Option<Dot> {
    block
        .descendants()
        .filter_map(|c| match c {
            ChildView::Leaf(l) => Some(l.dot()),
            ChildView::Block(_) => None,
        })
        .last()
}

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
        ChildView::Block(b) => last_leaf_dot(&b).or_else(|| b.dot())?,
    };

    Some((first, last))
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_resource::{FontFamily, FontFamilySource, FontWeight, Resource};

    use super::*;
    use crate::test_utils::*;

    fn make_resource(families: impl IntoIterator<Item = (&'static str, Vec<u16>)>) -> Resource {
        let mut resource = Resource::new_test();
        resource.set_fonts(
            families
                .into_iter()
                .map(|(name, weights)| FontFamily {
                    name: name.to_string(),
                    source: FontFamilySource::Default,
                    weights: weights
                        .into_iter()
                        .map(|value| FontWeight {
                            value,
                            hash: format!("{name}-{value}"),
                            chunks: vec![vec![0x0000, 0xFFFF]],
                        })
                        .collect(),
                })
                .collect(),
        );
        resource
    }

    #[test]
    fn normalizes_unavailable_weight() {
        let resource = make_resource([("Source", vec![400, 700]), ("LightFont", vec![100, 300])]);
        let (initial, ..) = state! {
            doc {
                root [font_family("Source".to_string()), font_weight(400)] {
                    p1: paragraph { text("hello") [font_weight(700)] }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| set_font_family(
            &mut tr,
            "LightFont".to_string(),
            &resource
        ));
        let (expected, ..) = state! {
            doc {
                root [font_family("Source".to_string()), font_weight(400)] {
                    p1: paragraph {
                        text("hello") [font_weight(300), font_family("LightFont".to_string()), bold]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn converts_bold_marker_to_weight() {
        let resource = make_resource([("OldFont", vec![400]), ("NewFont", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root [font_family("OldFont".to_string()), font_weight(400)] {
                    p1: paragraph {
                        text("hello") [font_weight(400), font_family("OldFont".to_string()), bold]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| set_font_family(
            &mut tr,
            "NewFont".to_string(),
            &resource
        ));
        let (expected, ..) = state! {
            doc {
                root [font_family("OldFont".to_string()), font_weight(400)] {
                    p1: paragraph {
                        text("hello") [font_weight(700), font_family("NewFont".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn normalizes_collapsed_pending_weight() {
        let resource = make_resource([("Source", vec![400, 700]), ("LightFont", vec![100, 300])]);
        let (initial, ..) = state! {
            doc {
                root [font_family("Source".to_string()), font_weight(400)] {
                    p1: paragraph { text("hello") [font_weight(700)] }
                }
            }
            selection: (p1, 2)
        };
        let mut tr = Transaction::new(&initial);

        assert!(set_font_family(&mut tr, "LightFont".to_string(), &resource).unwrap());

        let pending = tr.state().pending_modifiers.as_slice();
        assert_eq!(pending.len(), 3);
        assert!(pending.iter().any(|pm| matches!(
            pm,
            PendingModifier::Set {
                modifier: Modifier::FontFamily { value }
            } if value == "LightFont"
        )));
        assert!(pending.iter().any(|pm| matches!(
            pm,
            PendingModifier::Set {
                modifier: Modifier::FontWeight { value: 300 }
            }
        )));
        assert!(pending.iter().any(|pm| matches!(
            pm,
            PendingModifier::Set {
                modifier: Modifier::Bold
            }
        )));
    }

    #[test]
    fn ignores_unavailable_family() {
        let resource = make_resource([("KnownFont", vec![400, 700])]);
        let (initial, ..) = state! {
            doc {
                root { p1: paragraph { text("hello") } }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut tr = Transaction::new(&initial);
        let changed = set_font_family(&mut tr, "UnknownFont".to_string(), &resource).unwrap();
        assert!(!changed);
        assert_state_eq!(tr.state(), &initial);
    }
}
