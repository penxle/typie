use editor_model::{Modifier, ModifierType, NodeRef, Schema};
use editor_state::{PendingModifier, PendingModifiers, Position};
use editor_transaction::Transaction;

use crate::helpers::{
    apply_modifier_to_node, collect_applicable_targets_in_range, collect_text_nodes_in_range,
    compact_and_restore_selection, filter_applicable_node_ids, is_text_applicable, is_unit_variant,
    resolve_applicable_target_collapsed,
};
use crate::{CommandError, CommandResult};

pub fn set_modifier(tr: &mut Transaction, modifier: Modifier) -> CommandResult {
    if is_unit_variant(&modifier) {
        return Err(CommandError::InvalidArgument(format!(
            "{:?} is a unit modifier, use toggle_modifier instead",
            modifier.as_type()
        )));
    }

    let modifier_type = modifier.as_type();
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let collapsed = selection.is_collapsed();
    let text_applicable = is_text_applicable(modifier_type);

    match (collapsed, text_applicable) {
        (true, true) => set_modifier_collapsed_text(tr, &modifier),
        (true, false) => set_modifier_collapsed_block(tr, &modifier),
        (false, true) => set_modifier_range_text(tr, &modifier),
        (false, false) => set_modifier_range_block(tr, &modifier),
    }
}

fn set_modifier_collapsed_text(tr: &mut Transaction, modifier: &Modifier) -> CommandResult {
    let modifier_type = modifier.as_type();
    let pos = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    // Value already provided to this node without an explicit override of its own
    // (the node's applied style ref / type-implicit, else style-aware ancestor
    // inheritance). Style-aware so a value a run's style already supplies is
    // recognized as "already applied" and not re-pended as an explicit Set.
    let provided_value = provided_without_explicit_value(&node, modifier_type);
    let has_explicit_override = node
        .explicit_modifiers()
        .any(|m| m.as_type() == modifier_type);

    let mut pending: PendingModifiers = tr
        .pending_modifiers()
        .iter()
        .filter(|pm| pm.as_type() != modifier_type)
        .cloned()
        .collect();

    if provided_value.as_ref() == Some(modifier) {
        // Desired value is already supplied by style/inheritance. Only drop a
        // differing explicit override so the node falls back to it.
        if has_explicit_override {
            pending.push(PendingModifier::Unset { ty: modifier_type });
        }
    } else {
        pending.push(PendingModifier::Set {
            modifier: modifier.clone(),
        });
    }

    tr.set_pending_modifiers(pending)?;
    Ok(true)
}

/// The modifier value in effect at `node` for `ty`, ignoring `node`'s own explicit
/// override: the node's applied style ref or type-implicit value (style modifiers
/// are only expanded on inline run nodes by `modifiers_with_style`), else the
/// nearest style-aware inheritable ancestor value. Used solely by the collapsed
/// pending decision; it must NOT feed insertion carry-over, which stays explicit-only.
fn provided_without_explicit_value(node: &NodeRef, ty: ModifierType) -> Option<Modifier> {
    let has_explicit = node.explicit_modifiers().any(|m| m.as_type() == ty);
    let mut local = node.modifiers_with_style().filter(|m| m.as_type() == ty);
    // `modifiers_with_style` yields explicit, then style, then implicit. Skip the
    // explicit entry so only the style/implicit contribution is considered here.
    let local_value = if has_explicit {
        local.nth(1)
    } else {
        local.next()
    };
    if let Some(m) = local_value {
        return Some(m.clone());
    }

    if !Schema::modifier_spec(ty).inheritable {
        return None;
    }
    for ancestor in node.ancestors().skip(1) {
        if let Some(m) = ancestor.modifiers_with_style().find(|m| m.as_type() == ty) {
            return Some(m.clone());
        }
    }
    None
}

fn set_modifier_collapsed_block(tr: &mut Transaction, modifier: &Modifier) -> CommandResult {
    let modifier_type = modifier.as_type();
    let pos = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;
    let doc = tr.doc();

    let Some(target) = resolve_applicable_target_collapsed(&doc, pos.node_id, modifier_type) else {
        return Ok(false);
    };
    apply_modifier_to_node(tr, &target, modifier)?;
    Ok(true)
}

fn set_modifier_range_text(tr: &mut Transaction, modifier: &Modifier) -> CommandResult {
    let selection = tr.selection().expect("entry caller guaranteed selection");
    let doc = tr.doc();
    let resolved = selection
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
    let from = Position::from(resolved.from());
    let to = Position::from(resolved.to());

    let node_ids = collect_text_nodes_in_range(tr, &from, &to)?;
    let applicable_node_ids = filter_applicable_node_ids(&tr.doc(), &node_ids, modifier.as_type());

    if applicable_node_ids.is_empty() {
        return Ok(false);
    }

    for &node_id in &applicable_node_ids {
        let doc = tr.doc();
        let node = doc
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;
        apply_modifier_to_node(tr, &node, modifier)?;
    }

    compact_and_restore_selection(tr, &node_ids)?;

    Ok(true)
}

fn set_modifier_range_block(tr: &mut Transaction, modifier: &Modifier) -> CommandResult {
    let modifier_type = modifier.as_type();
    let selection = tr.selection().expect("entry caller guaranteed selection");
    let doc = tr.doc();
    let resolved = selection
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;

    let targets: Vec<_> = collect_applicable_targets_in_range(&doc, &resolved, modifier_type)
        .into_iter()
        .map(|n| n.id())
        .collect();

    if targets.is_empty() {
        return Ok(false);
    }

    for target_id in targets {
        let doc = tr.doc();
        let target = doc
            .node(target_id)
            .ok_or(CommandError::NodeNotFound(target_id))?;
        apply_modifier_to_node(tr, &target, modifier)?;
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn collapsed_set_font_size() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
            pending_modifiers: [font_size(2400)]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_same_as_inherited_unsets() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") [font_size(2400)] }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 1600 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") [font_size(2400)] }
                }
            }
            selection: (t1, 3)
            pending_modifiers: [!font_size]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_text_color() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::TextColor {
                value: "#ff0000".to_string()
            }
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
            pending_modifiers: [text_color("#ff0000".to_string())]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_text_color_matching_root_default_at_empty_paragraph_is_noop() {
        let (initial, ..) = state! {
            doc {
                root [paragraph_indent(200)] {
                    p1: paragraph {}
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::TextColor {
                value: "black".to_string()
            }
        ));
        let (expected, ..) = state! {
            doc {
                root [paragraph_indent(200)] {
                    p1: paragraph {}
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_unit_variant_rejected() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let err = transact_err!(initial, |tr| set_modifier(&mut tr, Modifier::Italic));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn range_set_font_size() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("Hello")
                        t2: text("World")
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("HelloWorld") [font_size(2400)]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_same_as_inherited_removes() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("Hello") [font_size(2400)]
                        t2: text("World") [font_size(2400)]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 1600 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("HelloWorld")
                    }
                }
            }
            selection: (t1, 0) -> (t1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_replaces_existing() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("Hello") [font_size(2400)]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 3200 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("Hello") [font_size(3200)]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_partial_selection_splits() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("HelloWorld")
                    }
                }
            }
            selection: (t1, 2) -> (t1, 7)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        text("He")
                        t1: text("lloWo") [font_size(2400)]
                        text("rld")
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_font_size_skips_fold_title_applies_to_paragraph() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    fold {
                        fold_title { t1: text("Title") }
                        fold_content { paragraph { t2: text("Body") } }
                    }
                }
            }
            selection: (t1, 0) -> (t2, 4)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    fold {
                        fold_title { t1: text("Title") }
                        fold_content { paragraph { t2: text("Body") [font_size(2400)] } }
                    }
                }
            }
            selection: (t1, 0) -> (t2, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_font_size_on_fold_title_only_is_noop() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    fold {
                        fold_title { t1: text("Title") }
                        fold_content { paragraph { text("Body") } }
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact_fail!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
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
    fn collapsed_set_matching_run_style_value_is_noop_or_unset() {
        use editor_model::{Modifier, PlainStyleEntry};
        let (initial, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let mut setup = editor_transaction::Transaction::new(&initial);
        setup
            .set_style(
                "s1".into(),
                Some(PlainStyleEntry {
                    name: "s".into(),
                    modifiers: vec![Modifier::FontSize { value: 2400 }]
                        .into_iter()
                        .collect(),
                }),
            )
            .unwrap();
        setup.set_node_style(t1, Some("s1".into())).unwrap();
        let (with_style, ..) = setup.commit();

        // run already has font_size 2400 via its style → setting the same value must NOT add a pending Set
        let (actual, ..) = transact!(with_style, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        assert!(
            !actual.pending_modifiers.iter().any(|pm| matches!(
                pm,
                editor_state::PendingModifier::Set {
                    modifier: Modifier::FontSize { value: 2400 }
                }
            )),
            "matching style-provided value must not be added as pending Set"
        );
    }

    #[test]
    fn collapsed_set_preserves_other_pending() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
            pending_modifiers: [italic]
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
            pending_modifiers: [italic, font_size(2400)]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_line_height_applies_to_paragraph() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::LineHeight { value: 200 }
        ));
        let (expected, ..) = state! {
            doc { root { paragraph [line_height(200)] { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_line_height_replaces_existing() {
        let (initial, ..) = state! {
            doc { root { paragraph [line_height(150)] { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::LineHeight { value: 200 }
        ));
        let (expected, ..) = state! {
            doc { root { paragraph [line_height(200)] { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_line_height_on_hr_returns_false() {
        let (initial, ..) = state! {
            doc { root { hr: horizontal_rule {} paragraph { t1: text("Hello") } } }
            selection: (hr, 0)
        };
        transact_fail!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::LineHeight { value: 160 }
        ));
    }

    #[test]
    fn collapsed_set_block_gap_applies_to_root() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::BlockGap { value: 150 }
        ));
        let (expected, ..) = state! {
            doc { root [block_gap(150)] { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_line_height_across_two_paragraphs() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("Hello") }
                paragraph { t2: text("World") }
            } }
            selection: (t1, 2) -> (t2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::LineHeight { value: 180 }
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph [line_height(180)] { t1: text("Hello") }
                paragraph [line_height(180)] { t2: text("World") }
            } }
            selection: (t1, 2) -> (t2, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_line_height_partial_overlap_within_one_paragraph() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::LineHeight { value: 175 }
        ));
        let (expected, ..) = state! {
            doc { root { paragraph [line_height(175)] { t1: text("Hello") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_font_size_spanning_tab_applies_to_text_without_panic() {
        let (initial, t1, t2) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("Hello")
                        tab {}
                        t2: text("World")
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let actual_doc = actual.doc;
        let t1_entry = actual_doc.get_entry(t1).unwrap();
        let t2_entry = actual_doc.get_entry(t2).unwrap();
        assert!(
            t1_entry
                .modifiers
                .contains_key(&editor_model::ModifierType::FontSize)
                || t2_entry
                    .modifiers
                    .contains_key(&editor_model::ModifierType::FontSize),
            "font_size must be stamped on at least one text node"
        );
    }

    fn tab_has_modifier(doc: &editor_model::Doc, ty: editor_model::ModifierType) -> bool {
        doc.root().unwrap().descendants().any(|n| {
            matches!(n.node(), editor_model::Node::Tab(_))
                && n.explicit_modifiers().any(|m| m.as_type() == ty)
        })
    }

    fn tab_font_size(doc: &editor_model::Doc) -> Option<u32> {
        doc.root().unwrap().descendants().find_map(|n| {
            if matches!(n.node(), editor_model::Node::Tab(_)) {
                n.explicit_modifiers().find_map(|m| match m {
                    Modifier::FontSize { value } => Some(*value),
                    _ => None,
                })
            } else {
                None
            }
        })
    }

    #[test]
    fn range_set_font_size_stamps_tab() {
        let (initial, t1, t2) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("a")
                        tab {}
                        t2: text("b")
                    }
                }
            }
            selection: (t1, 0) -> (t2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        assert_eq!(
            tab_font_size(&actual.doc),
            Some(2400),
            "tab must carry the range font_size"
        );
        assert!(
            actual
                .doc
                .get_entry(t1)
                .unwrap()
                .modifiers
                .contains_key(&editor_model::ModifierType::FontSize),
            "leading text node must carry font_size"
        );
        assert!(
            actual
                .doc
                .get_entry(t2)
                .unwrap()
                .modifiers
                .contains_key(&editor_model::ModifierType::FontSize),
            "trailing text node must carry font_size"
        );
    }

    #[test]
    fn range_set_font_size_stamps_tab_undo_removes() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("a")
                        tab {}
                        t2: text("b")
                    }
                }
            }
            selection: (t1, 0) -> (t2, 1)
        };
        let initial_doc = initial.doc.clone();
        let (after_set, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        assert_eq!(tab_font_size(&after_set.doc), Some(2400));

        let revert =
            editor_transaction::build_revert_transaction(&after_set, &initial_doc).unwrap();
        let (reverted, ..) = revert.commit();
        assert!(
            !tab_has_modifier(&reverted.doc, editor_model::ModifierType::FontSize),
            "undo must remove the tab's font_size"
        );
    }
}
