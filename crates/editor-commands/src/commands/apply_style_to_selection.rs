use editor_model::ModifierType;
use editor_state::PendingStyle;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{
    capture_style_entry, clear_inline_modifier_types_in_selection, collect_run_nodes_in_selection,
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

    let style_modifier_types: Vec<ModifierType> = capture_style_entry(tr.state(), &style_id)
        .map(|entry| entry.modifiers.iter().map(|m| m.as_type()).collect())
        .unwrap_or_default();

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
