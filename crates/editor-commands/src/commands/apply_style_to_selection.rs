use editor_crdt::Dot;
use editor_model::{Modifier, ModifierType};
use editor_state::{PendingStyle, leaf_groups_in_range};
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{
    capture_style_entry, clear_inline_modifier_types_in_selection, collect_run_nodes_in_selection,
    is_text_applicable,
};

pub fn apply_style_to_selection(tr: &mut Transaction, style_id: String) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    if selection.anchor == selection.head {
        tr.set_pending_style(Some(PendingStyle::Set { style_id }))?;
        return Ok(true);
    }

    let run_dots = collect_run_nodes_in_selection(tr)?;
    if run_dots.is_empty() {
        return Ok(false);
    }

    let style_modifiers: Vec<Modifier> = capture_style_entry(tr.state(), &style_id)
        .map(|entry| entry.modifiers.into_iter().collect())
        .unwrap_or_default();
    let style_modifier_types: Vec<ModifierType> =
        style_modifiers.iter().map(|m| m.as_type()).collect();

    let mut changed = false;
    for elem in &run_dots {
        let cur = tr.state().projected.node_styles().value_of(*elem);
        if cur.as_deref() == Some(style_id.as_str()) {
            continue;
        }
        tr.set_node_style(*elem, Some(style_id.clone()))?;
        changed = true;
    }

    if !style_modifier_types.is_empty()
        && clear_inline_modifier_types_in_selection(tr, &style_modifier_types)?
    {
        changed = true;
    }

    // With the style attached and inline Sets of its types cancelled above, the only
    // way a style-provided type can still be missing from a run's effective map is an
    // older explicit clear blocking it. Cancel those too, so the style shows through.
    let blocked: Vec<(Dot, Dot, Modifier)> = {
        let view = tr.state().view();
        match selection.resolve(&view) {
            Some(rs) => leaf_groups_in_range(&rs)
                .into_iter()
                .flat_map(|g| {
                    style_modifiers
                        .iter()
                        .filter(|m| {
                            is_text_applicable(m.as_type())
                                && !g.effective.contains_key(&m.as_type())
                        })
                        .map(move |m| (g.first, g.last, m.clone()))
                        .collect::<Vec<_>>()
                })
                .collect(),
            None => Vec::new(),
        }
    };
    for (first, last, modifier) in blocked {
        tr.remove_span_modifier(first, last, modifier)?;
        changed = true;
    }

    Ok(changed)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;

    use super::*;
    use crate::commands::define_style;
    use crate::test_utils::*;

    #[test]
    fn applies_style_to_selected_runs_only_not_whole_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("HelloWorld") } } }
            selection: (p1, 2) -> (p1, 7)
        };
        let (with_style, ..) = transact!(initial, |tr| crate::commands::define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![editor_model::Modifier::Bold]
        ));
        let (actual, ..) = transact!(with_style, |tr| apply_style_to_selection(
            &mut tr,
            "h1".into()
        ));

        let (expected, ..) = state! {
            doc {
                styles { h1: "제목" [bold] }
                root { p1: paragraph {
                    text("He")
                    text("lloWo") @h1
                    text("rld")
                } }
            }
            selection: (p1, 2) -> (p1, 7)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn applies_style_to_last_run_with_upstream_block_end_head() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0, >) -> (p1, 5, <)
        };
        let (with_style, ..) = transact!(initial, |tr| crate::commands::define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![editor_model::Modifier::Bold]
        ));
        let (actual, ..) = transact!(with_style, |tr| apply_style_to_selection(
            &mut tr,
            "h1".into()
        ));

        let (expected, ..) = state! {
            doc {
                styles { h1: "제목" [bold] }
                root { p1: paragraph { text("Hello") @h1 } }
            }
            selection: (p1, 0, >) -> (p1, 5, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_apply_sets_pending_style() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let (with_style, ..) = transact!(initial, |tr| crate::commands::define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![]
        ));
        let (actual, ..) = transact!(with_style, |tr| apply_style_to_selection(
            &mut tr,
            "h1".into()
        ));
        assert_eq!(
            actual.pending_style,
            Some(editor_state::PendingStyle::Set {
                style_id: "h1".into()
            })
        );
    }

    #[test]
    fn clears_inline_modifiers_of_style_types_in_range() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [font_size(800)] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (with_style, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![Modifier::FontSize { value: 2800 }],
        ));
        let (actual, ..) = transact!(with_style, |tr| apply_style_to_selection(
            &mut tr,
            "h1".into()
        ));

        let (expected, ..) = state! {
            doc {
                styles { h1: "제목" [font_size(2800)] }
                root { p1: paragraph { text("Hello") @h1 } }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn style_provided_color_stays_effective_after_inline_clear() {
        use editor_model::ChildView;

        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("Hello") [text_color("#ffff00".to_string())] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (with_style, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "c1".into(),
            "노랑".into(),
            vec![Modifier::TextColor {
                value: "#ffff00".to_string()
            }],
        ));
        let (actual, ..) = transact!(with_style, |tr| apply_style_to_selection(
            &mut tr,
            "c1".into()
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
                .get(&ModifierType::TextColor),
            Some(&Modifier::TextColor {
                value: "#ffff00".to_string()
            }),
            "style-provided TextColor must survive the inline-span cleanup"
        );
    }

    #[test]
    fn reapplying_style_cancels_explicit_clear() {
        use editor_model::ChildView;

        let (initial, p1) = state! {
            doc {
                styles { em: "강조" [italic] }
                root { p1: paragraph { text("Hello") @em } }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (toggled, ..) = transact!(initial, |tr| crate::commands::toggle_modifier(
            &mut tr,
            ModifierType::Italic
        ));
        let (actual, ..) = transact!(toggled, |tr| apply_style_to_selection(&mut tr, "em".into()));

        let view = actual.view();
        let node = view.node(p1).unwrap();
        let Some(ChildView::Leaf(_leaf)) = node.child_at(0) else {
            panic!("expected leaf at offset 0");
        };
        assert_eq!(
            node.leaf_state_at(0)
                .unwrap()
                .eff
                .get(&ModifierType::Italic),
            Some(&editor_model::Modifier::Italic),
            "re-applying the style must cancel the earlier explicit clear"
        );
    }

    #[test]
    fn preserves_inline_modifiers_outside_style_types() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [text_color("#ff0000".to_string())] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (with_style, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![Modifier::FontSize { value: 2800 }],
        ));
        let (actual, ..) = transact!(with_style, |tr| apply_style_to_selection(
            &mut tr,
            "h1".into()
        ));

        let (expected, ..) = state! {
            doc {
                styles { h1: "제목" [font_size(2800)] }
                root { p1: paragraph { text("Hello") @h1 [text_color("#ff0000".to_string())] } }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }
}
