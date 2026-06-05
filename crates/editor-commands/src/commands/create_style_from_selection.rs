use editor_model::{ModifierType, PlainStyleEntry};
use editor_transaction::Transaction;

use crate::helpers::{
    clear_inline_modifier_types_in_selection, collect_uniform_text_modifiers_in_selection,
};
use crate::{CommandError, CommandResult};

pub fn create_style_from_selection(
    tr: &mut Transaction,
    style_id: String,
    name: String,
) -> CommandResult {
    if style_id.is_empty() {
        return Err(CommandError::InvalidArgument(
            "style_id must not be empty".into(),
        ));
    }

    let run_ids = crate::helpers::collect_run_nodes_in_selection(tr)?;
    if run_ids.is_empty() {
        return Ok(false);
    }

    let modifiers = collect_uniform_text_modifiers_in_selection(tr.state());
    let modifier_types: Vec<ModifierType> = modifiers.iter().map(|m| m.as_type()).collect();

    tr.set_style(
        style_id.clone(),
        Some(PlainStyleEntry {
            name,
            modifiers: modifiers.into_iter().collect(),
        }),
    )?;

    for node_id in &run_ids {
        tr.set_node_style(*node_id, Some(style_id.clone()))?;
    }

    clear_inline_modifier_types_in_selection(tr, &modifier_types)?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn captures_uniform_inline_modifiers_into_new_style() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [font_size(800), text_color("#ff00ff".to_string())] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| create_style_from_selection(
            &mut tr,
            "s1".into(),
            "핑크임".into(),
        ));

        let style = actual.doc.style_entry("s1").unwrap();
        assert_eq!(style.name.get(), "핑크임");
        let mods: Vec<Modifier> = style.modifiers.iter().cloned().collect();
        assert!(mods.contains(&Modifier::FontSize { value: 800 }));
        assert!(mods.contains(&Modifier::TextColor {
            value: "#ff00ff".into()
        }));
    }

    #[test]
    fn sets_style_ref_on_selected_runs() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [font_size(800)] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| create_style_from_selection(
            &mut tr,
            "s1".into(),
            "x".into()
        ));
        let para = actual.doc.root().unwrap().children().next().unwrap();
        assert!(
            para.children()
                .any(|c| c.entry().style.get().as_deref() == Some("s1"))
        );
        assert_eq!(
            para.entry().style.get().as_deref(),
            None,
            "paragraph must not carry style"
        );
    }

    #[test]
    fn clears_inline_modifiers_moved_into_style() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [font_size(800), text_color("#ff00ff".to_string())] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| create_style_from_selection(
            &mut tr,
            "s1".into(),
            "x".into(),
        ));
        let para = actual.doc.root().unwrap().children().next().unwrap();
        let text = para.children().next().unwrap();
        let explicit: Vec<Modifier> = text.explicit_modifiers().cloned().collect();
        assert!(
            !explicit
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { .. })),
            "font_size should be moved into style"
        );
        assert!(
            !explicit
                .iter()
                .any(|m| matches!(m, Modifier::TextColor { .. })),
            "text_color should be moved into style"
        );
    }

    #[test]
    fn drops_mixed_modifiers() {
        let (initial, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [font_size(800)]
                t2: text("World") [font_size(1600)]
            } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| create_style_from_selection(
            &mut tr,
            "s1".into(),
            "x".into(),
        ));
        let style = actual.doc.style_entry("s1").unwrap();
        let mods: Vec<Modifier> = style.modifiers.iter().cloned().collect();
        assert!(
            !mods.iter().any(|m| matches!(m, Modifier::FontSize { .. })),
            "mixed font_size should be dropped from style"
        );
    }

    #[test]
    fn registers_style_presence_on_root() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| create_style_from_selection(
            &mut tr,
            "s1".into(),
            "x".into(),
        ));
        assert!(actual.doc.style_present("s1"));
    }

    #[test]
    fn empty_style_id_errors() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let err = transact_err!(initial, |tr| create_style_from_selection(
            &mut tr,
            "".into(),
            "x".into(),
        ));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }
}
