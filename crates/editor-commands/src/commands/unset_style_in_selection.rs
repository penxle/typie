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

    if selection.is_collapsed() {
        tr.set_pending_style(Some(PendingStyle::Unset))?;
        return Ok(true);
    }

    let run_ids = collect_run_nodes_in_selection(tr)?;
    if run_ids.is_empty() {
        return Ok(false);
    }

    let mut changed = false;
    for node_id in &run_ids {
        if tr
            .state()
            .doc
            .get_entry(*node_id)
            .and_then(|e| e.style.get().clone())
            .is_none()
        {
            continue;
        }
        tr.set_node_style(*node_id, None)?;
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
    use editor_model::Modifier;

    use super::*;
    use crate::commands::apply_style_to_selection;
    use crate::test_utils::*;

    #[test]
    fn unset_clears_style_on_selected_runs() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("HelloWorld") } } }
            selection: (t1, 0) -> (t1, 10)
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
        let para = actual.doc.root().unwrap().children().next().unwrap();
        assert!(para.children().all(|c| c.entry().style.get().is_none()));
    }

    #[test]
    fn collapsed_unset_sets_pending_unset() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
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
                p1: paragraph { t1: text("Hello") }
                p2: paragraph { t2: text("World") }
            } }
            selection: (t1, 0) -> (t2, 5)
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
        let root = actual.doc.root().unwrap();
        for para in root.children() {
            for run in para.children() {
                assert!(
                    run.entry().style.get().is_none(),
                    "run {:?} should have no style after unset",
                    run.id()
                );
            }
        }
    }

    #[test]
    fn clears_inline_font_size_in_range() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [font_size(800)] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| unset_style_in_selection(&mut tr));
        let para = actual.doc.root().unwrap().children().next().unwrap();
        let text = para.children().next().unwrap();
        let has_font_size = text
            .explicit_modifiers()
            .any(|m| matches!(m, Modifier::FontSize { .. }));
        assert!(!has_font_size, "inline font_size should be cleared");
    }

    #[test]
    fn preserves_text_color_when_clearing_font_size() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [font_size(800), text_color("#ff0000".to_string())] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| unset_style_in_selection(&mut tr));
        let para = actual.doc.root().unwrap().children().next().unwrap();
        let text = para.children().next().unwrap();
        let has_font_size = text
            .explicit_modifiers()
            .any(|m| matches!(m, Modifier::FontSize { .. }));
        let has_color = text
            .explicit_modifiers()
            .any(|m| matches!(m, Modifier::TextColor { .. }));
        assert!(!has_font_size, "font_size should be cleared");
        assert!(has_color, "text_color should be preserved");
    }
}
