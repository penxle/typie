use editor_crdt::Dot;
use editor_model::Modifier;
use editor_state::{PendingModifier, PendingModifiers, caret_provided_and_override};
use editor_transaction::Transaction;

use crate::helpers::{
    apply_modifier_to_node, collect_applicable_targets_in_range, is_table_justify, is_unit_variant,
    resolve_applicable_target_collapsed, set_modifier_range_text,
};
use crate::{CommandError, CommandResult};

pub fn set_modifier(tr: &mut Transaction, modifier: Modifier) -> CommandResult {
    if is_unit_variant(&modifier) {
        return Err(CommandError::InvalidArgument(format!(
            "{:?} is a unit modifier, use toggle_modifier instead",
            modifier.as_type()
        )));
    }
    if !modifier.is_valid() {
        return Ok(false);
    }

    let modifier_type = modifier.as_type();
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let collapsed = selection.anchor == selection.head;
    let text_applicable = modifier_type.is_text_applicable();

    match (collapsed, text_applicable) {
        (true, true) => set_modifier_collapsed_text(tr, &modifier),
        (true, false) => set_modifier_collapsed_block(tr, &modifier),
        (false, true) => set_modifier_range_text(tr, selection, &modifier),
        (false, false) => set_modifier_range_block(tr, &modifier),
    }
}

fn set_modifier_collapsed_text(tr: &mut Transaction, modifier: &Modifier) -> CommandResult {
    let modifier_type = modifier.as_type();
    if !modifier_type.is_carry_kind() {
        return Ok(false);
    }
    let pos = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;

    let (provided_value, has_explicit_override) = {
        let view = tr.view();
        view.node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
        caret_provided_and_override(&tr.state().projected, pos.node, pos.offset, modifier_type)
    };

    let mut pending: PendingModifiers = tr
        .pending_modifiers()
        .iter()
        .filter(|pm| pm.as_type() != modifier_type)
        .cloned()
        .collect();

    if provided_value.as_ref() == Some(modifier) {
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

fn set_modifier_collapsed_block(tr: &mut Transaction, modifier: &Modifier) -> CommandResult {
    let modifier_type = modifier.as_type();
    let pos = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;

    let target = {
        let view = tr.view();
        resolve_applicable_target_collapsed(&view, pos.node, modifier_type)
    };
    let Some(target) = target else {
        return Ok(false);
    };
    let skip = {
        let view = tr.view();
        is_table_justify(&view, target, modifier)
    };
    if skip {
        return Ok(false);
    }
    apply_modifier_to_node(tr, target, modifier)?;
    Ok(true)
}

fn set_modifier_range_block(tr: &mut Transaction, modifier: &Modifier) -> CommandResult {
    let modifier_type = modifier.as_type();
    let selection = tr.selection().expect("entry caller guaranteed selection");

    let targets: Vec<Dot> = {
        let view = tr.view();
        let rs = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        collect_applicable_targets_in_range(&view, &rs, modifier_type)
            .into_iter()
            .filter(|&id| !is_table_justify(&view, id, modifier))
            .collect()
    };

    if targets.is_empty() {
        return Ok(false);
    }

    for target in targets {
        apply_modifier_to_node(tr, target, modifier)?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;
    use editor_model::ModifierType;

    #[test]
    fn collapsed_set_font_size() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 3)
            pending_modifiers: [font_size(2400)]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_same_as_inherited_unsets() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("Hello") [font_size(2400)] }
                }
            }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 1600 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("Hello") [font_size(2400)] }
                }
            }
            selection: (p1, 3)
            pending_modifiers: [!font_size]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_empty_paragraph_carry_set_to_inherited_unsets() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph carry([font_size(1600)]) {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 1200 }
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph carry([font_size(1600)]) {}
                }
            }
            selection: (p1, 0)
            pending_modifiers: [!font_size]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_after_trailing_page_break_reads_left_charlike_override() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("가") [font_size(1600)]
                        page_break
                    }
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 1200 }
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("가") [font_size(1600)]
                        page_break
                    }
                }
            }
            selection: (p1, 2)
            pending_modifiers: [!font_size]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_text_color() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 3)
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
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 3)
            pending_modifiers: [text_color("#ff0000".to_string())]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_font_size_matching_root_default_at_empty_paragraph_is_noop() {
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
            Modifier::FontSize { value: 1200 }
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
    fn collapsed_set_link_returns_false_without_pending() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact_fail!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::Link {
                href: "https://a.com".to_string()
            }
        ));
        assert!(
            actual.pending_modifiers.is_empty(),
            "collapsed link must not create pending"
        );
    }

    #[test]
    fn collapsed_set_ruby_returns_false_without_pending() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact_fail!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::Ruby {
                text: "x".to_string()
            }
        ));
        assert!(
            actual.pending_modifiers.is_empty(),
            "collapsed ruby must not create pending"
        );
    }

    #[test]
    fn collapsed_set_unit_variant_rejected() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let err = transact_err!(initial, |tr| set_modifier(&mut tr, Modifier::Italic));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn range_set_font_size() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("HelloWorld") }
                }
            }
            selection: (p1, 0) -> (p1, 10)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph carry([font_size(2400)]) { text("HelloWorld") [font_size(2400)] }
                }
            }
            selection: (p1, 0) -> (p1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_same_as_inherited_removes() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("HelloWorld") [font_size(2400)] }
                }
            }
            selection: (p1, 0) -> (p1, 10)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 1600 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("HelloWorld") }
                }
            }
            selection: (p1, 0) -> (p1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_same_as_inherited_shows_inherited_value() {
        use editor_model::ChildView;

        let (initial, p1) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("HelloWorld") [font_size(2400)] }
                }
            }
            selection: (p1, 0) -> (p1, 10)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 1600 }
        ));

        let view = actual.view();
        let node = view.node(p1).unwrap();
        let Some(ChildView::Leaf(_leaf)) = node.child_at(0) else {
            panic!("expected leaf at offset 0");
        };
        assert_eq!(
            node.leaf_state_at(0)
                .unwrap()
                .eff
                .get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 }),
            "cancelling the override must fall back to the inherited value, not None"
        );
    }

    #[test]
    fn range_set_replaces_existing() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("Hello") [font_size(2400)] }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 3200 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph carry([font_size(3200)]) { text("Hello") [font_size(3200)] }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_partial_applies_span_to_substring() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p: paragraph { text("HelloWorld") }
                }
            }
            selection: (p, 2) -> (p, 7)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p: paragraph {
                        text("He")
                        text("lloWo") [font_size(2400)]
                        text("rld")
                    }
                }
            }
            selection: (p, 2) -> (p, 7)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_font_size_ending_at_empty_paragraph_start_applies_to_selected_text() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("12") }
                    p2: paragraph {}
                }
            }
            selection: (p1, 1, >) -> (p2, 0, <)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph carry([font_size(2400)]) {
                        text("1")
                        text("2") [font_size(2400)]
                    }
                    p2: paragraph {}
                }
            }
            selection: (p1, 1, >) -> (p2, 0, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_alignment_ending_at_empty_paragraph_start_applies_to_both_paragraphs() {
        use editor_model::Alignment;
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("hello") }
                    p2: paragraph {}
                }
            }
            selection: (p1, 0, >) -> (p2, 0, <)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::Alignment {
                value: Alignment::Center
            }
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph [alignment(Alignment::Center)] { text("hello") }
                    p2: paragraph [alignment(Alignment::Center)] {}
                }
            }
            selection: (p1, 0, >) -> (p2, 0, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_font_size_across_two_paragraphs_at_block_level() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("hello") }
                paragraph { text("world") }
            } }
            selection: (r, 0, >) -> (r, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc { r: root {
                paragraph carry([font_size(2400)]) { text("hello") [font_size(2400)] }
                paragraph carry([font_size(2400)]) { text("world") [font_size(2400)] }
            } }
            selection: (r, 0, >) -> (r, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_preserves_other_pending() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 3)
            pending_modifiers: [italic]
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 3)
            pending_modifiers: [italic, font_size(2400)]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_line_height_applies_to_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::LineHeight { value: 200 }
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph [line_height(200)] { text("Hello") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_line_height_replaces_existing() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph [line_height(150)] { text("Hello") } } }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::LineHeight { value: 200 }
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph [line_height(200)] { text("Hello") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_line_height_on_hr_returns_false() {
        let (initial, ..) = state! {
            doc { root { hr: horizontal_rule {} p1: paragraph { text("Hello") } } }
            selection: (hr, 0)
        };
        transact_fail!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::LineHeight { value: 160 }
        ));
    }

    #[test]
    fn set_modifier_block_gap_is_noop_root_only() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::BlockGap { value: 150 }
        ));
    }

    #[test]
    fn range_set_line_height_across_two_paragraphs() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("Hello") }
                p2: paragraph { text("World") }
            } }
            selection: (p1, 2) -> (p2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::LineHeight { value: 180 }
        ));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph [line_height(180)] { text("Hello") }
                p2: paragraph [line_height(180)] { text("World") }
            } }
            selection: (p1, 2) -> (p2, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_line_height_partial_overlap_within_one_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::LineHeight { value: 175 }
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph [line_height(175)] { text("Hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn apply_alignment_on_cell_paragraph_writes_record_no_table_inheritance() {
        use editor_model::Alignment;
        let (initial, p) = state! {
            doc { root {
                table [alignment(Alignment::Right)] {
                    table_row {
                        table_cell { p: paragraph { text("x") } }
                    }
                }
            } }
            selection: (p, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::Alignment {
                value: Alignment::Right
            }
        ));
        let (expected, ..) = state! {
            doc { root {
                table [alignment(Alignment::Right)] {
                    table_row {
                        table_cell { p: paragraph [alignment(Alignment::Right)] { text("x") } }
                    }
                }
            } }
            selection: (p, 0)
        };
        let _ = p;
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_out_of_range_font_size_is_noop() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("HelloWorld") }
                }
            }
            selection: (p1, 0) -> (p1, 10)
        };
        transact_fail!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 399 }
        ));
    }

    #[test]
    fn collapsed_set_table_justify_is_noop() {
        use editor_model::Alignment;
        let (initial, table) = state! {
            doc { root {
                table: table {
                    table_row {
                        table_cell { paragraph { text("x") } }
                    }
                }
            } }
            selection: (table, 0)
        };
        let _ = table;
        transact_fail!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::Alignment {
                value: Alignment::Justify
            }
        ));
    }

    #[test]
    fn range_set_justify_across_paragraph_and_table_applies_to_paragraph_only() {
        use editor_model::{Alignment, ModifierType};

        let (initial, r, p1, table) = state! {
            doc { r: root {
                p1: paragraph { text("hello") }
                table: table {
                    table_row {
                        table_cell { paragraph { text("x") } }
                    }
                }
            } }
            selection: (r, 0, >) -> (r, 2, <)
        };
        let _ = r;
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::Alignment {
                value: Alignment::Justify
            }
        ));

        let view = actual.view();
        assert_eq!(
            view.node(p1)
                .unwrap()
                .block_modifier(ModifierType::Alignment),
            Some(&Modifier::Alignment {
                value: Alignment::Justify
            })
        );
        assert_eq!(
            view.node(table)
                .unwrap()
                .block_modifier(ModifierType::Alignment),
            None,
            "table alignment target is skipped for justify"
        );
    }

    #[test]
    fn cross_paragraph_records_carry_on_both_when_end_touched() {
        let (initial, p1, p2) = state! {
            doc { root {
                p1: paragraph { text("Hello") }
                p2: paragraph { text("World") }
            } }
            selection: (p1, 2) -> (p2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        assert_eq!(
            actual
                .projected
                .carry_modifiers(p1)
                .get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 2400 }),
            "p1 is selected through its end (middle-block rule)"
        );
        assert_eq!(
            actual
                .projected
                .carry_modifiers(p2)
                .get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 2400 }),
            "p2 end is touched"
        );
    }

    #[test]
    fn cross_paragraph_second_middle_leaves_its_carry_untouched() {
        let (initial, p1, p2) = state! {
            doc { root {
                p1: paragraph { text("Hello") }
                p2: paragraph { text("World") }
            } }
            selection: (p1, 2) -> (p2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        assert_eq!(
            actual
                .projected
                .carry_modifiers(p1)
                .get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 2400 })
        );
        assert!(
            actual.projected.carry_modifiers(p2).is_empty(),
            "selection ends in the middle of p2 → its carry is untouched"
        );
    }

    #[test]
    fn empty_middle_paragraph_records_carry() {
        let (initial, _p1, p2, _p3) = state! {
            doc { root {
                p1: paragraph { text("Hello") }
                p2: paragraph {}
                p3: paragraph { text("World") }
            } }
            selection: (p1, 0) -> (p3, 5)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        assert_eq!(
            actual
                .projected
                .carry_modifiers(p2)
                .get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 2400 }),
            "an empty paragraph strictly inside the range records carry"
        );
    }

    #[test]
    fn range_set_to_inherited_removes_carry_at_end() {
        let (initial, p1) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph carry([font_size(2400)]) {
                        text("Hello") [font_size(2400)]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 1600 }
        ));
        assert!(
            !actual
                .projected
                .carry_modifiers(p1)
                .contains_key(&ModifierType::FontSize),
            "setting the block's inherited value unsets carry"
        );
    }

    #[test]
    fn cell_rect_set_font_size_applies_to_rect_cells_only() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    table {
                        tr1: table_row {
                            table_cell { paragraph { text("1") } }
                            table_cell { paragraph { text("2") } }
                        }
                        tr2: table_row {
                            table_cell { paragraph { text("3") } }
                            table_cell { paragraph { text("4") } }
                        }
                    }
                }
            }
            selection: (tr1, 0, >) -> (tr2, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    table {
                        tr1: table_row {
                            table_cell {
                                paragraph carry([font_size(2400)]) { text("1") [font_size(2400)] }
                            }
                            table_cell { paragraph { text("2") } }
                        }
                        tr2: table_row {
                            table_cell {
                                paragraph carry([font_size(2400)]) { text("3") [font_size(2400)] }
                            }
                            table_cell { paragraph { text("4") } }
                        }
                    }
                }
            }
            selection: (tr1, 0, >) -> (tr2, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn cell_rect_set_line_height_applies_to_rect_cell_paragraphs_only() {
        let (initial, ..) = state! {
            doc { root {
                table {
                    tr1: table_row {
                        table_cell { paragraph { text("1") } }
                        table_cell { paragraph { text("2") } }
                    }
                    tr2: table_row {
                        table_cell { paragraph { text("3") } }
                        table_cell { paragraph { text("4") } }
                    }
                }
            } }
            selection: (tr1, 0, >) -> (tr2, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::LineHeight { value: 180 }
        ));
        let (expected, ..) = state! {
            doc { root {
                table {
                    tr1: table_row {
                        table_cell { paragraph [line_height(180)] { text("1") } }
                        table_cell { paragraph { text("2") } }
                    }
                    tr2: table_row {
                        table_cell { paragraph [line_height(180)] { text("3") } }
                        table_cell { paragraph { text("4") } }
                    }
                }
            } }
            selection: (tr1, 0, >) -> (tr2, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn cell_rect_records_carry_on_all_cell_paragraphs_including_empty() {
        use editor_state::{State, cell_rect_selection};

        let (initial, r0c0, pa, pb, pc, r1c1, pd) = state! {
            doc { root {
                table {
                    table_row {
                        r0c0: table_cell { pa: paragraph { text("A") } }
                        table_cell { pb: paragraph {} }
                    }
                    table_row {
                        table_cell { pc: paragraph { text("C") } }
                        r1c1: table_cell { pd: paragraph { text("D") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let sel = {
            let view = initial.view();
            cell_rect_selection(r0c0, r1c1, &view).unwrap()
        };
        let initial = State {
            selection: Some(sel),
            ..initial
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 1400 }
        ));
        for p in [pa, pb, pc, pd] {
            assert_eq!(
                actual
                    .projected
                    .carry_modifiers(p)
                    .get(&ModifierType::FontSize),
                Some(&Modifier::FontSize { value: 1400 }),
                "every paragraph in the selected cells (empty included) records carry"
            );
        }
    }
}
