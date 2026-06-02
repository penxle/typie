use editor_model::ModifierType;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{clear_inline_modifier_types_in_selection, collect_textblocks_in_selection};

pub fn apply_style_to_selection(tr: &mut Transaction, style_id: String) -> CommandResult {
    let textblock_ids = collect_textblocks_in_selection(tr.state());
    if textblock_ids.is_empty() {
        return Ok(false);
    }

    let style_modifier_types: Vec<ModifierType> = {
        let doc = tr.doc();
        doc.style_entry(&style_id)
            .map(|entry| entry.modifiers.iter().map(|m| m.as_type()).collect())
            .unwrap_or_default()
    };

    let mut changed = false;
    for node_id in textblock_ids {
        let Some(entry) = tr.state().doc.get_entry(node_id) else {
            continue;
        };
        if entry.style.get().as_deref() == Some(style_id.as_str()) {
            continue;
        }
        tr.set_node_style(node_id, Some(style_id.clone()))?;
        changed = true;
    }

    if clear_inline_modifier_types_in_selection(tr, &style_modifier_types)? {
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
    fn applies_style_to_all_textblocks_in_range() {
        let (initial, p1, p2, ..) = state! {
            doc { root {
                p1: paragraph { t1: text("Hello") }
                p2: paragraph { t2: text("World") }
            } }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| apply_style_to_selection(&mut tr, "h1".into()));
        assert_eq!(
            actual.doc.get_entry(p1).unwrap().style.get().as_deref(),
            Some("h1")
        );
        assert_eq!(
            actual.doc.get_entry(p2).unwrap().style.get().as_deref(),
            Some("h1")
        );
    }

    #[test]
    fn applies_style_to_collapsed_textblock() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| apply_style_to_selection(&mut tr, "h1".into()));
        assert_eq!(
            actual.doc.get_entry(p1).unwrap().style.get().as_deref(),
            Some("h1")
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
        let text = para.children().next().unwrap();
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
