use editor_model::Modifier;
use editor_state::{PendingModifier, PendingModifiers, Position};
use editor_transaction::Transaction;

use crate::helpers::{
    apply_modifier_to_node, collect_applicable_targets_in_range, collect_text_nodes_in_range,
    compact_and_restore_selection, filter_applicable_node_ids, is_text_applicable, is_unit_variant,
    resolve_applicable_target_collapsed, resolve_base_modifiers, resolve_inherited_modifiers,
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

    let inherited = resolve_inherited_modifiers(&node);
    let inherited_value = inherited.iter().find(|m| m.as_type() == modifier_type);

    let mut pending: PendingModifiers = tr
        .pending_modifiers()
        .iter()
        .filter(|pm| pm.as_type() != modifier_type)
        .cloned()
        .collect();

    if inherited_value == Some(modifier) {
        let base_has_override = resolve_base_modifiers(&node, pos.offset)
            .iter()
            .any(|m| m.as_type() == modifier_type);
        if base_has_override {
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
}
