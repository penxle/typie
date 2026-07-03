use editor_crdt::Dot;
use editor_model::{ChildView, Modifier, ModifierType};
use editor_transaction::Transaction;

use crate::helpers::{
    capture_style_entry, clear_inline_modifier_types_in_selection, collect_run_nodes_in_selection,
    collect_textblocks_in_selection, collect_uniform_text_modifiers_in_selection,
};
use crate::{CommandError, CommandResult};

pub fn update_style_from_selection(tr: &mut Transaction) -> CommandResult {
    let Some(style_id) = current_style_id(tr) else {
        return Ok(false);
    };

    let Some(mut entry) = capture_style_entry(tr.state(), &style_id) else {
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

fn current_style_id(tr: &mut Transaction) -> Option<String> {
    let run_dots = collect_run_nodes_in_selection(tr).ok()?;
    if !run_dots.is_empty() {
        let styles = tr.state().projected.node_styles();
        let mut iter = run_dots.iter().map(|elem| styles.value_of(*elem));
        if let Some(first) = iter.next() {
            if iter.clone().all(|s| s == first) {
                if let Some(style_id) = first {
                    return Some(style_id);
                }
            } else {
                return None;
            }
        }
    }

    let textblock_ids = collect_textblocks_in_selection(tr.state());
    if textblock_ids.is_empty() {
        return None;
    }
    let mut canonical: Option<String> = None;
    for id in &textblock_ids {
        let style = tr.state().projected.node_styles().value_of(*id);
        match (style, &canonical) {
            (Some(s), None) => canonical = Some(s),
            (Some(s), Some(c)) if &s == c => {}
            _ => return None,
        }
    }
    canonical
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
    if selection.anchor != selection.head {
        return clear_inline_modifier_types_in_selection(tr, types);
    }

    let to_remove: Vec<(Dot, Modifier)> = {
        let view = tr.state().view();
        let pos = &selection.head;
        let Some(node) = view.node(pos.node) else {
            return Ok(false);
        };
        let idx = pos.offset.checked_sub(1);
        let leaf_slot = match idx {
            Some(i) if node.child_at(i).is_some() => i,
            _ => pos.offset,
        };
        let Some(ChildView::Leaf(l)) = node.child_at(leaf_slot) else {
            return Ok(false);
        };
        let Some(st) = node.leaf_state_at(leaf_slot) else {
            return Ok(false);
        };
        let mut acc = Vec::new();
        for (ty, own) in st.own {
            if own.from_style {
                continue;
            }
            if types.contains(ty) {
                acc.push((l.dot(), own.value.clone()));
            }
        }
        acc
    };

    let mut changed = false;
    for (elem, modifier) in to_remove {
        if let Some(op) = elem.as_op_dot() {
            let dot = op.dot();
            tr.remove_span_modifier(dot, dot, modifier)?;
            changed = true;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::commands::{apply_style_to_selection, define_style};
    use crate::test_utils::*;

    #[test]
    fn merges_uniform_inline_modifier_into_style() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("Hello") [font_size(2400)] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![],
        ));
        let (applied, ..) = transact!(defined, |tr| apply_style_to_selection(&mut tr, "h1".into()));

        let (actual, ..) = transact!(applied, |tr| update_style_from_selection(&mut tr));

        let (expected, _p1) = state! {
            doc {
                styles { h1: "제목" [font_size(2400)] }
                root { p1: paragraph { text("Hello") @h1 } }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);

        let style = capture_style_entry(&actual, "h1").unwrap();
        assert!(
            style
                .modifiers
                .contains(&Modifier::FontSize { value: 2400 }),
            "inline FontSize should be merged into style"
        );
    }

    #[test]
    fn replaces_same_type_modifier_value_in_style() {
        let (initial, p, ..) = state! {
            doc { root { p: paragraph { text("Hello") [font_size(2400)] } } }
            selection: (p, 0) -> (p, 5)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![Modifier::FontSize { value: 1600 }],
        ));
        let mut tr = Transaction::new(&defined);
        tr.set_node_style(p, Some("h1".into())).unwrap();
        let (applied, ..) = tr.commit();

        let (actual, ..) = transact!(applied, |tr| update_style_from_selection(&mut tr));

        let style = capture_style_entry(&actual, "h1").unwrap();
        assert!(
            style
                .modifiers
                .contains(&Modifier::FontSize { value: 2400 })
        );
        assert!(
            !style
                .modifiers
                .contains(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn clears_inline_modifiers_for_collapsed_caret() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { text("H") [font_size(2400)] } } }
            selection: (p1, 1)
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

        let style = capture_style_entry(&actual, "h1").unwrap();
        assert!(
            style
                .modifiers
                .contains(&Modifier::FontSize { value: 2400 })
        );

        let (expected, _p) = state! {
            doc {
                styles { h1: "제목" [font_size(2400)] }
                root { p: paragraph @h1 { text("H") } }
            }
            selection: (p, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn clears_inline_modifiers_after_merge() {
        let (initial, p, ..) = state! {
            doc { root { p: paragraph { text("Hello") [font_size(2400)] } } }
            selection: (p, 0) -> (p, 5)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![],
        ));
        let mut tr = Transaction::new(&defined);
        tr.set_node_style(p, Some("h1".into())).unwrap();
        let (applied, ..) = tr.commit();

        let (actual, ..) = transact!(applied, |tr| update_style_from_selection(&mut tr));

        let (expected, _p) = state! {
            doc {
                styles { h1: "제목" [font_size(2400)] }
                root { p: paragraph @h1 { text("Hello") } }
            }
            selection: (p, 0) -> (p, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn noop_when_no_style_applied() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("Hello") [font_size(2400)] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact_fail!(initial, |tr| update_style_from_selection(&mut tr));

        let (expected, _p1) = state! {
            doc { root { p1: paragraph { text("Hello") [font_size(2400)] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn noop_when_styles_mixed_across_selection() {
        let (initial, p1, p2, ..) = state! {
            doc { root {
                p1: paragraph { text("Foo") [font_size(2400)] }
                p2: paragraph { text("Bar") [font_size(2400)] }
            } }
            selection: (p1, 0) -> (p2, 3)
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
        let style_a = capture_style_entry(&actual, "a").unwrap();
        let style_b = capture_style_entry(&actual, "b").unwrap();
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
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("Hello") [font_family("Arial".to_string())] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "제목".into(),
            vec![],
        ));
        let (applied, ..) = transact!(defined, |tr| apply_style_to_selection(&mut tr, "h1".into()));

        let (actual, ..) = transact!(applied, |tr| update_style_from_selection(&mut tr));

        let style = capture_style_entry(&actual, "h1").unwrap();
        assert!(
            style.modifiers.contains(&Modifier::FontFamily {
                value: "Arial".to_string()
            }),
            "inline FontFamily should be merged into style, got: {:?}",
            style.modifiers
        );

        let (expected, _p1) = state! {
            doc {
                styles { h1: "제목" [font_family("Arial".to_string())] }
                root { p1: paragraph { text("Hello") @h1 } }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);

        let view = actual.view();
        let node = view.node(_p1).unwrap();
        let Some(ChildView::Leaf(_leaf)) = node.child_at(0) else {
            panic!("expected leaf at offset 0");
        };
        assert_eq!(
            node.leaf_state_at(0)
                .unwrap()
                .eff
                .get(&ModifierType::FontFamily),
            Some(&Modifier::FontFamily {
                value: "Arial".to_string()
            }),
            "cancelling the merged inline span must let the style value show"
        );
    }

    #[test]
    fn replaces_font_family_value_in_style() {
        let (initial, p, ..) = state! {
            doc { root { p: paragraph { text("Hello") [font_family("Arial".to_string())] } } }
            selection: (p, 0) -> (p, 5)
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
        tr.set_node_style(p, Some("h1".into())).unwrap();
        let (applied, ..) = tr.commit();

        let (actual, ..) = transact!(applied, |tr| update_style_from_selection(&mut tr));

        let style = capture_style_entry(&actual, "h1").unwrap();
        assert!(
            style.modifiers.contains(&Modifier::FontFamily {
                value: "Arial".to_string()
            }),
            "style should have new FontFamily Arial, got: {:?}",
            style.modifiers
        );
        assert!(
            !style.modifiers.contains(&Modifier::FontFamily {
                value: "Pretendard".to_string()
            }),
            "old FontFamily Pretendard should be replaced, got: {:?}",
            style.modifiers
        );

        let (expected, _p) = state! {
            doc {
                styles { h1: "제목" [font_family("Arial".to_string())] }
                root { p: paragraph @h1 { text("Hello") } }
            }
            selection: (p, 0) -> (p, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn font_family_propagates_to_other_styled_nodes() {
        let (initial, p1, p2) = state! {
            doc { root {
                p1: paragraph { text("Foo") [font_family("Arial".to_string())] }
                p2: paragraph { text("Bar") }
            } }
            selection: (p1, 0) -> (p1, 3)
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
        tr.set_node_style(p1, Some("h1".into())).unwrap();
        tr.set_node_style(p2, Some("h1".into())).unwrap();
        let (applied, ..) = tr.commit();

        let (actual, ..) = transact!(applied, |tr| update_style_from_selection(&mut tr));

        // FontFamily targets Paragraph > Text, so it surfaces on the text leaf
        // (not the paragraph's own effective); verify the cascade there.
        let view = actual.view();
        let p2_node = view.node(p2).unwrap();
        let ChildView::Leaf(_leaf) = p2_node.child_at(0).unwrap() else {
            panic!("expected text leaf under the other styled paragraph");
        };
        assert_eq!(
            p2_node
                .leaf_state_at(0)
                .unwrap()
                .eff
                .get(&ModifierType::FontFamily),
            Some(&Modifier::FontFamily {
                value: "Arial".to_string()
            }),
            "text under the other styled node should see updated FontFamily through cascade"
        );
    }

    #[test]
    fn skips_non_uniform_modifier_type() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph {
                text("Foo") [font_size(2000)]
                text("Bar") [font_size(2400)]
            } } }
            selection: (p1, 0) -> (p1, 6)
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

        let style = capture_style_entry(&actual, "h1").unwrap();
        assert!(
            style
                .modifiers
                .contains(&Modifier::FontSize { value: 1600 }),
            "non-uniform inline FontSize should not change style"
        );
    }

    #[test]
    fn updates_style_referenced_by_selected_runs() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (defined, ..) = transact!(initial, |tr| crate::commands::define_style(
            &mut tr,
            "h1".into(),
            "x".into(),
            vec![]
        ));
        let (applied, ..) = transact!(defined, |tr| crate::commands::apply_style_to_selection(
            &mut tr,
            "h1".into()
        ));
        let (sized, ..) = transact!(applied, |tr| crate::commands::set_modifier(
            &mut tr,
            editor_model::Modifier::FontSize { value: 2800 }
        ));
        let (actual, ..) = transact!(sized, |tr| update_style_from_selection(&mut tr));
        let style = capture_style_entry(&actual, "h1").unwrap();
        assert!(
            style
                .modifiers
                .iter()
                .any(|m| matches!(m, editor_model::Modifier::FontSize { value: 2800 }))
        );
    }
}
