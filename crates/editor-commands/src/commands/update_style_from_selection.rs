use editor_model::{Modifier, ModifierType, Node};
use editor_transaction::Transaction;

use crate::helpers::{
    capture_style_entry, clear_inline_modifier_types_in_selection, collect_textblocks_in_selection,
    collect_uniform_text_modifiers_in_selection,
};
use crate::{CommandError, CommandResult};

pub fn update_style_from_selection(tr: &mut Transaction) -> CommandResult {
    let textblock_ids = collect_textblocks_in_selection(tr.state());
    if textblock_ids.is_empty() {
        return Ok(false);
    }

    let mut canonical: Option<String> = None;
    for id in &textblock_ids {
        let Some(entry) = tr.state().doc.get_entry(*id) else {
            return Ok(false);
        };
        let style = entry.style.get().clone();
        match (style, &canonical) {
            (Some(s), None) => canonical = Some(s),
            (Some(s), Some(c)) if &s == c => {}
            _ => return Ok(false),
        }
    }
    let Some(style_id) = canonical else {
        return Ok(false);
    };

    let Some(mut entry) = capture_style_entry(&tr.state().doc, &style_id) else {
        return Ok(false);
    };

    let uniform_inline = collect_uniform_text_modifiers_in_selection(tr.state());
    if uniform_inline.is_empty() {
        return Ok(false);
    }

    let mut applied_types: Vec<ModifierType> = Vec::new();
    let mut style_changed = false;
    for modifier in uniform_inline {
        let ty = modifier.as_type();
        if entry.modifiers.contains(&modifier) {
            applied_types.push(ty);
            continue;
        }
        let before_len = entry.modifiers.len();
        entry.modifiers.retain(|m| m.as_type() != ty);
        let removed_any = entry.modifiers.len() != before_len;
        let inserted = entry.modifiers.insert(modifier);
        if inserted || removed_any {
            style_changed = true;
        }
        applied_types.push(ty);
    }

    if style_changed {
        tr.set_style(style_id, Some(entry))?;
    }

    let cleared = clear_applied_inline(tr, &applied_types)?;
    Ok(style_changed || cleared)
}

fn clear_applied_inline(
    tr: &mut Transaction,
    types: &[ModifierType],
) -> Result<bool, CommandError> {
    if types.is_empty() {
        return Ok(false);
    }
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return clear_inline_modifier_types_in_selection(tr, types);
    }

    let node_id = selection.head.node_id;
    let to_remove: Vec<Modifier> = {
        let doc = tr.doc();
        let Some(node) = doc.node(node_id) else {
            return Ok(false);
        };
        if !matches!(node.node(), Node::Text(_)) {
            return Ok(false);
        }
        node.explicit_modifiers()
            .filter(|m| types.contains(&m.as_type()))
            .cloned()
            .collect()
    };

    let mut changed = false;
    for modifier in to_remove {
        tr.remove_modifier(node_id, modifier)?;
        changed = true;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;

    use super::*;
    use crate::commands::{apply_style_to_selection, define_style};
    use crate::test_utils::*;

    #[test]
    fn merges_uniform_inline_modifier_into_style() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [font_size(2400)] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![],
        ));
        let (applied, ..) = transact!(defined, |tr| apply_style_to_selection(&mut tr, "h1".into()));

        let para = applied.doc.root().unwrap().children().next().unwrap();
        let text = para.children().next().unwrap();
        let still_has_inline = text
            .explicit_modifiers()
            .any(|m| matches!(m, Modifier::FontSize { .. }));
        assert!(
            still_has_inline,
            "precondition: inline font_size kept because style had no font_size",
        );

        let (actual, ..) = transact!(applied, |tr| update_style_from_selection(&mut tr));

        let style = actual.doc.style_entry("h1").unwrap();
        let mods: Vec<Modifier> = style.modifiers.iter().cloned().collect();
        assert!(
            mods.contains(&Modifier::FontSize { value: 2400 }),
            "inline FontSize should be merged into style"
        );
    }

    #[test]
    fn replaces_same_type_modifier_value_in_style() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [font_size(2400)] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![Modifier::FontSize { value: 1600 }],
        ));
        let mut tr = Transaction::new(&defined);
        let para_id = defined.doc.root().unwrap().children().next().unwrap().id();
        tr.set_node_style(para_id, Some("h1".into())).unwrap();
        let (applied, ..) = tr.commit();

        let (actual, ..) = transact!(applied, |tr| update_style_from_selection(&mut tr));

        let style = actual.doc.style_entry("h1").unwrap();
        let mods: Vec<Modifier> = style.modifiers.iter().cloned().collect();
        assert!(mods.contains(&Modifier::FontSize { value: 2400 }));
        assert!(!mods.contains(&Modifier::FontSize { value: 1600 }));
    }

    #[test]
    fn clears_inline_modifiers_for_collapsed_caret() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") [font_size(2400)] } } }
            selection: (t1, 2)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![],
        ));
        let mut tr = Transaction::new(&defined);
        tr.set_node_style(p1, Some("h1".into())).unwrap();
        let (applied, ..) = tr.commit();

        let (actual, ..) = transact!(applied, |tr| update_style_from_selection(&mut tr));

        let style = actual.doc.style_entry("h1").unwrap();
        assert!(
            style
                .modifiers
                .contains(&Modifier::FontSize { value: 2400 })
        );

        let para = actual.doc.root().unwrap().children().next().unwrap();
        let text = para.children().next().unwrap();
        let has_inline = text
            .explicit_modifiers()
            .any(|m| matches!(m, Modifier::FontSize { .. }));
        assert!(
            !has_inline,
            "inline font_size should be cleared on caret text node",
        );
    }

    #[test]
    fn clears_inline_modifiers_after_merge() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [font_size(2400)] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![],
        ));
        let mut tr = Transaction::new(&defined);
        let para_id = defined.doc.root().unwrap().children().next().unwrap().id();
        tr.set_node_style(para_id, Some("h1".into())).unwrap();
        let (applied, ..) = tr.commit();

        let (actual, ..) = transact!(applied, |tr| update_style_from_selection(&mut tr));

        let para = actual.doc.root().unwrap().children().next().unwrap();
        let text = para.children().next().unwrap();
        let has_inline = text
            .explicit_modifiers()
            .any(|m| matches!(m, Modifier::FontSize { .. }));
        assert!(
            !has_inline,
            "inline font_size should be cleared after merge"
        );
    }

    #[test]
    fn noop_when_no_style_applied() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [font_size(2400)] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact_fail!(initial, |tr| update_style_from_selection(&mut tr));
        let para = actual.doc.root().unwrap().children().next().unwrap();
        let text = para.children().next().unwrap();
        assert!(
            text.explicit_modifiers()
                .any(|m| matches!(m, Modifier::FontSize { .. })),
            "inline font_size preserved when no style to update"
        );
    }

    #[test]
    fn noop_when_styles_mixed_across_selection() {
        let (initial, p1, p2, ..) = state! {
            doc { root {
                p1: paragraph { t1: text("Foo") [font_size(2400)] }
                p2: paragraph { t2: text("Bar") [font_size(2400)] }
            } }
            selection: (t1, 0) -> (t2, 3)
        };
        let (defined1, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "a".into(),
            "A".into(),
            vec![],
        ));
        let (defined2, ..) = transact!(defined1, |tr| define_style(
            &mut tr,
            "b".into(),
            "B".into(),
            vec![],
        ));
        let mut tr = Transaction::new(&defined2);
        tr.set_node_style(p1, Some("a".into())).unwrap();
        tr.set_node_style(p2, Some("b".into())).unwrap();
        let (mixed, ..) = tr.commit();

        let (actual, ..) = transact_fail!(mixed, |tr| update_style_from_selection(&mut tr));
        let style_a = actual.doc.style_entry("a").unwrap();
        let style_b = actual.doc.style_entry("b").unwrap();
        assert!(
            !style_a
                .modifiers
                .contains(&Modifier::FontSize { value: 2400 })
        );
        assert!(
            !style_b
                .modifiers
                .contains(&Modifier::FontSize { value: 2400 })
        );
    }

    #[test]
    fn merges_uniform_inline_font_family_into_style() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [font_family("Arial".to_string())] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![],
        ));
        let (applied, ..) = transact!(defined, |tr| apply_style_to_selection(&mut tr, "h1".into()));

        let (actual, ..) = transact!(applied, |tr| update_style_from_selection(&mut tr));

        let style = actual.doc.style_entry("h1").unwrap();
        let mods: Vec<Modifier> = style.modifiers.iter().cloned().collect();
        assert!(
            mods.contains(&Modifier::FontFamily {
                value: "Arial".to_string()
            }),
            "inline FontFamily should be merged into style, got: {mods:?}"
        );

        let para = actual.doc.root().unwrap().children().next().unwrap();
        let text = para.children().next().unwrap();
        let has_inline = text
            .explicit_modifiers()
            .any(|m| matches!(m, Modifier::FontFamily { .. }));
        assert!(
            !has_inline,
            "inline FontFamily should be cleared after merge"
        );

        let effective_on_para: Vec<&Modifier> = para
            .modifiers_with_style()
            .filter(|m| matches!(m, Modifier::FontFamily { .. }))
            .collect();
        assert_eq!(
            effective_on_para,
            vec![&Modifier::FontFamily {
                value: "Arial".to_string()
            }],
            "paragraph (the styled node) resolves FontFamily through its style"
        );
    }

    #[test]
    fn replaces_font_family_value_in_style() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [font_family("Arial".to_string())] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![Modifier::FontFamily {
                value: "Pretendard".to_string(),
            }],
        ));
        let mut tr = Transaction::new(&defined);
        let para_id = defined.doc.root().unwrap().children().next().unwrap().id();
        tr.set_node_style(para_id, Some("h1".into())).unwrap();
        let (applied, ..) = tr.commit();

        let (actual, ..) = transact!(applied, |tr| update_style_from_selection(&mut tr));

        let style = actual.doc.style_entry("h1").unwrap();
        let mods: Vec<Modifier> = style.modifiers.iter().cloned().collect();
        assert!(
            mods.contains(&Modifier::FontFamily {
                value: "Arial".to_string()
            }),
            "style should have new FontFamily Arial, got: {mods:?}"
        );
        assert!(
            !mods.contains(&Modifier::FontFamily {
                value: "Pretendard".to_string()
            }),
            "old FontFamily Pretendard should be replaced, got: {mods:?}"
        );

        let para = actual.doc.root().unwrap().children().next().unwrap();
        let text = para.children().next().unwrap();
        let has_inline = text
            .explicit_modifiers()
            .any(|m| matches!(m, Modifier::FontFamily { .. }));
        assert!(
            !has_inline,
            "inline FontFamily should be cleared after replace"
        );
    }

    #[test]
    fn font_family_propagates_to_other_styled_nodes() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("Foo") [font_family("Arial".to_string())] }
                paragraph { t2: text("Bar") }
            } }
            selection: (t1, 0) -> (t1, 3)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![Modifier::FontFamily {
                value: "Pretendard".to_string(),
            }],
        ));
        let p1 = defined.doc.root().unwrap().children().next().unwrap().id();
        let p2 = defined.doc.root().unwrap().children().nth(1).unwrap().id();
        let mut tr = Transaction::new(&defined);
        tr.set_node_style(p1, Some("h1".into())).unwrap();
        tr.set_node_style(p2, Some("h1".into())).unwrap();
        let (applied, ..) = tr.commit();

        let (actual, ..) = transact!(applied, |tr| update_style_from_selection(&mut tr));

        let para2 = actual.doc.node(p2).unwrap();
        let effective: Vec<&Modifier> = para2
            .modifiers_with_style()
            .filter(|m| matches!(m, Modifier::FontFamily { .. }))
            .collect();
        assert_eq!(
            effective,
            vec![&Modifier::FontFamily {
                value: "Arial".to_string()
            }],
            "other styled node should see updated FontFamily through cascade"
        );
    }

    #[test]
    fn skips_non_uniform_modifier_type() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph {
                t1: text("Foo") [font_size(2000)]
                t2: text("Bar") [font_size(2400)]
            } } }
            selection: (t1, 0) -> (t2, 3)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![Modifier::FontSize { value: 1600 }],
        ));
        let mut tr = Transaction::new(&defined);
        tr.set_node_style(p1, Some("h1".into())).unwrap();
        let (applied, ..) = tr.commit();

        let (actual, ..) = transact_fail!(applied, |tr| update_style_from_selection(&mut tr));

        let style = actual.doc.style_entry("h1").unwrap();
        assert!(
            style
                .modifiers
                .contains(&Modifier::FontSize { value: 1600 }),
            "non-uniform inline FontSize should not change style"
        );
    }
}
