use editor_crdt::Dot;
use editor_model::{DocView, Modifier, ModifierType};
use editor_state::{PendingModifier, PendingModifiers};
use editor_transaction::Transaction;

use crate::helpers::{
    apply_modifier_to_node, collect_applicable_targets_in_range, is_text_applicable,
    is_unit_variant, resolve_applicable_target_collapsed, span_dots,
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
    let collapsed = selection.anchor == selection.head;
    let text_applicable = is_text_applicable(modifier_type);

    match (collapsed, text_applicable) {
        (true, true) => set_modifier_collapsed_text(tr, &modifier),
        (true, false) => set_modifier_collapsed_block(tr, &modifier),
        (false, true) => set_modifier_range_text(tr, &modifier),
        (false, false) => set_modifier_range_block(tr, &modifier),
    }
}

/// The value at a collapsed caret supplied by the leaf's applied style or by
/// ancestor inheritance, ignoring any explicit (non-style) leaf override; and
/// whether such an explicit override is present.
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
    let st = node.leaf_state_at(leaf_idx);
    let own = st.and_then(|s| s.own.get(&ty).map(|o| (o.value.clone(), o.from_style)));
    let has_explicit_override = matches!(&own, Some((_, false)));
    let provided = match &own {
        // The leaf's own style supplies the value directly.
        Some((value, true)) => Some(value.clone()),
        // An explicit (non-style) override shadows the inherited value; report
        // the inherited value so a matching Set drops the override instead.
        Some((_, false)) => block_eff,
        // No own modifier: the leaf inherits the value (e.g. from an ancestor's
        // node style). Text-target modifiers like FontSize don't surface on the
        // block's own effective map, so read the leaf's effective value.
        None => st.and_then(|s| s.eff.get(&ty).cloned()).or(block_eff),
    };
    (provided, has_explicit_override)
}

fn set_modifier_collapsed_text(tr: &mut Transaction, modifier: &Modifier) -> CommandResult {
    let modifier_type = modifier.as_type();
    let pos = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;

    let (provided_value, has_explicit_override) = {
        let view = tr.view();
        view.node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
        provided_and_override(&view, pos.node, pos.offset, modifier_type)
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
    apply_modifier_to_node(tr, target, modifier)?;
    Ok(true)
}

fn set_modifier_range_text(tr: &mut Transaction, modifier: &Modifier) -> CommandResult {
    let modifier_type = modifier.as_type();
    let selection = tr.selection().expect("entry caller guaranteed selection");

    let (first, last, inherited_eq) = {
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
            .and_then(|n| n.effective().get(&modifier_type).cloned());
        (first, last, inherited.as_ref() == Some(modifier))
    };

    if inherited_eq {
        tr.remove_span_modifier(first, last, modifier.clone())?;
    } else {
        tr.add_span_modifier(first, last, modifier.clone())?;
    }
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
                    p1: paragraph { text("HelloWorld") [font_size(2400)] }
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
                    p1: paragraph { text("Hello") [font_size(3200)] }
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
                paragraph { text("hello") [font_size(2400)] }
                paragraph { text("world") [font_size(2400)] }
            } }
            selection: (r, 0, >) -> (r, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_matching_run_style_value_is_noop() {
        use editor_model::{Modifier, PlainStyleEntry};
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
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
        setup.set_node_style(p1, Some("s1".into())).unwrap();
        let (with_style, ..) = setup.commit();

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
    fn collapsed_set_block_gap_applies_to_root() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::BlockGap { value: 150 }
        ));
        let (expected, ..) = state! {
            doc { root [block_gap(150)] { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
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
    fn apply_alignment_writes_explicit_even_when_same_as_table() {
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
}
