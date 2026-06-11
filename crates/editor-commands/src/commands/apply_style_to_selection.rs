use editor_model::ModifierType;
use editor_state::PendingStyle;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{
    clear_inline_modifier_types_in_selection, collect_run_nodes_in_selection,
    compact_textblocks_for_nodes,
};

pub fn apply_style_to_selection(tr: &mut Transaction, style_id: String) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    if selection.is_collapsed() {
        tr.set_pending_style(Some(PendingStyle::Set { style_id }))?;
        return Ok(true);
    }

    let run_ids = collect_run_nodes_in_selection(tr)?;
    if run_ids.is_empty() {
        return Ok(false);
    }

    let style_modifier_types: Vec<ModifierType> = {
        let doc = tr.doc();
        doc.style_entry(&style_id)
            .map(|entry| entry.modifiers.iter().map(|m| m.as_type()).collect())
            .unwrap_or_default()
    };

    let mut changed = false;
    for node_id in &run_ids {
        let cur = tr
            .state()
            .doc
            .get_entry(*node_id)
            .and_then(|e| e.style.get().clone());
        if cur.as_deref() == Some(style_id.as_str()) {
            continue;
        }
        tr.set_node_style(*node_id, Some(style_id.clone()))?;
        changed = true;
    }

    if style_modifier_types.is_empty() {
        compact_textblocks_for_nodes(tr, &run_ids)?;
    } else if clear_inline_modifier_types_in_selection(tr, &style_modifier_types)? {
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
            doc { root { paragraph { t1: text("HelloWorld") } } }
            selection: (t1, 2) -> (t1, 7)
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

        let para = actual.doc.root().unwrap().children().next().unwrap();
        let runs: Vec<_> = para.children().collect();
        assert_eq!(runs.len(), 3, "selection boundary splits runs");
        assert_eq!(runs[0].entry().style.get().as_deref(), None);
        assert_eq!(runs[1].entry().style.get().as_deref(), Some("h1"));
        assert_eq!(runs[2].entry().style.get().as_deref(), None);
        assert_eq!(para.entry().style.get().as_deref(), None);
    }

    #[test]
    fn collapsed_apply_sets_pending_style() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
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
            doc { root { paragraph { t1: text("Hello") [font_size(800)] } } }
            selection: (t1, 0) -> (t1, 5)
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

        let para = actual.doc.root().unwrap().children().next().unwrap();
        let runs: Vec<_> = para.children().collect();
        assert_eq!(runs.len(), 1, "full-run selection needs no boundary splits");
        let text = &runs[0];
        assert_eq!(text.entry().style.get().as_deref(), Some("h1"));
        let has_font_size = text
            .explicit_modifiers()
            .any(|m| matches!(m, Modifier::FontSize { .. }));
        assert!(!has_font_size, "inline font_size should be cleared");
    }

    #[test]
    fn preserves_inline_modifiers_outside_style_types() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [text_color("#ff0000".to_string())] } } }
            selection: (t1, 0) -> (t1, 5)
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

        let para = actual.doc.root().unwrap().children().next().unwrap();
        let text = para.children().next().unwrap();
        let has_color = text
            .explicit_modifiers()
            .any(|m| matches!(m, Modifier::TextColor { .. }));
        assert!(
            has_color,
            "text_color outside style types should be preserved"
        );
    }
}
