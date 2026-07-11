use editor_common::Tri;
use editor_model::{DEFAULT_FONT_WEIGHT, Modifier, ModifierType};
use editor_resource::Resource;
use editor_state::{
    PendingModifier, PendingModifiers, caret_provided_and_override, resolve_modifier_state,
};
use editor_transaction::Transaction;

use crate::helpers::{font_weight, set_font_family_range, weight_and_bold_after_family_change};
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
        set_font_family_range(tr, selection, family, weights)
    }
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
        let (provided, explicit) = caret_provided_and_override(
            &tr.state().projected,
            pos.node,
            pos.offset,
            ModifierType::FontFamily,
        );
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
                    p1: paragraph carry([font_family("LightFont".to_string()), font_weight(300)]) {
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
                    p1: paragraph carry([font_family("NewFont".to_string())]) {
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
    fn empty_paragraph_carry_set_to_default_family_applies() {
        let resource = make_resource([("Pretendard", vec![400, 700]), ("RIDIBatang", vec![400])]);
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph carry([font_family("RIDIBatang".to_string())]) {
                        text("리디바탕") [font_family("RIDIBatang".to_string())]
                    }
                    p1: paragraph carry([font_family("RIDIBatang".to_string())]) {}
                }
            }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&initial);

        assert!(set_font_family(&mut tr, "Pretendard".to_string(), &resource).unwrap());

        let selection = tr.selection().unwrap();
        let ms = resolve_modifier_state(&tr.state().projected, &selection, tr.pending_modifiers())
            .unwrap();
        assert_eq!(
            ms.font_family,
            Tri::Uniform {
                value: editor_model::FontFamilyValue {
                    value: "Pretendard".to_string()
                }
            },
            "setting the default family over an empty-paragraph carry must surface at the caret"
        );

        assert!(crate::commands::insert_text(&mut tr, "가").unwrap());
        let head = tr.selection().unwrap().head;
        let typed_own = tr.view().node(head.node).unwrap().leaf_own_modifiers_at(0);
        assert!(
            !typed_own
                .iter()
                .any(|m| matches!(m, Modifier::FontFamily { .. })),
            "text typed after the change must not keep the carried family: {typed_own:?}"
        );
    }

    #[test]
    fn page_break_only_paragraph_carry_set_to_default_family_unsets() {
        let resource = make_resource([("Pretendard", vec![400, 700]), ("RIDIBatang", vec![400])]);
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph carry([font_family("RIDIBatang".to_string())]) { page_break }
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_font_family(
            &mut tr,
            "Pretendard".to_string(),
            &resource
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph carry([font_family("RIDIBatang".to_string())]) { page_break }
                }
            }
            selection: (p1, 0)
            pending_modifiers: [!font_family]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn caret_after_trailing_page_break_set_to_default_family_unsets() {
        let resource = make_resource([("Pretendard", vec![400, 700]), ("RIDIBatang", vec![400])]);
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("리") [font_family("RIDIBatang".to_string())]
                        page_break
                    }
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| set_font_family(
            &mut tr,
            "Pretendard".to_string(),
            &resource
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("리") [font_family("RIDIBatang".to_string())]
                        page_break
                    }
                }
            }
            selection: (p1, 2)
            pending_modifiers: [!font_family]
        };
        assert_state_eq!(&actual, &expected);
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
