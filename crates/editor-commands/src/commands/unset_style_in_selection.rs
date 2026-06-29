use editor_model::ModifierType;
use editor_state::PendingStyle;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{clear_inline_modifier_types_in_selection, collect_run_nodes_in_selection};

const DEFAULT_STYLE_TYPES: &[ModifierType] = &[ModifierType::FontSize, ModifierType::FontWeight];

pub fn unset_style_in_selection(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    if selection.anchor == selection.head {
        tr.set_pending_style(Some(PendingStyle::Unset))?;
        return Ok(true);
    }

    let run_dots = collect_run_nodes_in_selection(tr)?;
    if run_dots.is_empty() {
        return Ok(false);
    }

    let mut changed = false;
    for elem in &run_dots {
        let Some(op) = elem.as_op_dot() else { continue };
        if tr
            .state()
            .projected
            .node_styles()
            .value_of(op.dot())
            .is_none()
        {
            continue;
        }
        tr.set_node_style(*elem, None)?;
        changed = true;
    }

    if clear_inline_modifier_types_in_selection(tr, DEFAULT_STYLE_TYPES)? {
        changed = true;
    }

    Ok(changed)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::commands::apply_style_to_selection;
    use crate::test_utils::*;

    #[test]
    fn unset_clears_style_on_selected_runs() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("HelloWorld") } } }
            selection: (p1, 0) -> (p1, 10)
        };
        let (with_style, ..) = transact!(initial, |tr| crate::commands::define_style(
            &mut tr,
            "h1".into(),
            "x".into(),
            vec![]
        ));
        let (applied, ..) = transact!(with_style, |tr| apply_style_to_selection(
            &mut tr,
            "h1".into()
        ));
        let (actual, ..) = transact!(applied, |tr| unset_style_in_selection(&mut tr));

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("HelloWorld") } } }
            selection: (p1, 0) -> (p1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_unset_sets_pending_unset() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| unset_style_in_selection(&mut tr));
        assert_eq!(
            actual.pending_style,
            Some(editor_state::PendingStyle::Unset)
        );
    }

    #[test]
    fn clears_styles_on_all_textblocks_in_range() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("Hello") }
                p2: paragraph { text("World") }
            } }
            selection: (p1, 0) -> (p2, 5)
        };
        let (with_style, ..) = transact!(initial, |tr| crate::commands::define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![]
        ));
        let (applied, ..) = transact!(with_style, |tr| apply_style_to_selection(
            &mut tr,
            "h1".into()
        ));
        let (actual, ..) = transact!(applied, |tr| unset_style_in_selection(&mut tr));

        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("Hello") }
                p2: paragraph { text("World") }
            } }
            selection: (p1, 0) -> (p2, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn clears_inline_font_size_in_range() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [font_size(800)] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| unset_style_in_selection(&mut tr));

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn preserves_text_color_when_clearing_font_size() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [font_size(800), text_color("#ff0000".to_string())] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| unset_style_in_selection(&mut tr));

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [text_color("#ff0000".to_string())] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }
}
