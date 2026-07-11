use editor_commands::{self as commands, CommandResult};
use editor_model::{DocView, Modifier, ModifierType};
use editor_resource::Resource;
use editor_state::{
    Composition, PendingModifier, Position, ResolvedPosition, ResolvedPositionFlatExt, Selection,
    replacement_paint,
};
use editor_transaction::Transaction;
use std::sync::Arc;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

fn is_composition_routable(op: &ModifierOp) -> bool {
    match op {
        ModifierOp::Toggle { .. } | ModifierOp::ClearAll => true,
        ModifierOp::Set { modifier } => modifier.as_type().is_text_applicable(),
        ModifierOp::Edit { modifier_type, .. } => modifier_type.is_text_applicable(),
        ModifierOp::SetOnNode { .. } => false,
    }
}

fn composition_selection(view: &DocView, comp: Composition) -> Option<Selection> {
    let from = ResolvedPosition::from_flat(view, comp.start)?;
    let to = ResolvedPosition::from_flat(view, comp.end)?;
    Some(Selection::new((&from).into(), (&to).into()))
}

fn apply_char_format_in_selection(
    tr: &mut Transaction,
    range: Selection,
    op: &ModifierOp,
    resource: &Resource,
) -> CommandResult {
    match op {
        ModifierOp::Toggle {
            modifier_type: ModifierType::Bold,
        } => commands::toggle_bold_in_selection(tr, range, resource),
        ModifierOp::Toggle { modifier_type } => {
            commands::toggle_modifier_in_selection(tr, range, *modifier_type)
        }
        ModifierOp::Set {
            modifier: Modifier::FontFamily { value },
        } => commands::set_font_family_in_selection(tr, range, value.clone(), resource),
        ModifierOp::Set { modifier } => {
            commands::set_modifier_text_in_selection(tr, range, modifier.clone())
        }
        ModifierOp::Edit {
            modifier_type,
            modifier,
        } => commands::edit_modifier_in_selection(tr, range, *modifier_type, modifier.clone()),
        ModifierOp::ClearAll => commands::clear_all_modifiers_in_selection(tr, range),
        ModifierOp::SetOnNode { .. } => Ok(false),
    }
}

fn run_collapsed_format(
    tr: &mut Transaction,
    op: &ModifierOp,
    resource: &Resource,
) -> Result<(), EditorError> {
    match op {
        ModifierOp::Toggle {
            modifier_type: ModifierType::Bold,
        } => {
            commands::toggle_bold(tr, resource)?;
        }
        ModifierOp::Toggle { modifier_type } => {
            commands::toggle_modifier(tr, *modifier_type)?;
        }
        ModifierOp::Set {
            modifier: Modifier::FontFamily { value },
        } => {
            commands::set_font_family(tr, value.clone(), resource)?;
        }
        ModifierOp::Set { modifier } => {
            commands::set_modifier(tr, modifier.clone())?;
        }
        ModifierOp::Edit {
            modifier_type,
            modifier,
        } => {
            commands::edit_modifier(tr, *modifier_type, modifier.clone())?;
        }
        ModifierOp::ClearAll => {
            commands::clear_all_modifiers(tr)?;
        }
        ModifierOp::SetOnNode { .. } => {}
    }
    Ok(())
}

fn route_nonempty_composition_format(
    editor: &mut Editor,
    op: ModifierOp,
    comp: Composition,
) -> Result<(), EditorError> {
    let resource = Arc::clone(&editor.resource);
    let resource = resource.lock().unwrap();
    editor.transact(|tr| {
        let Some(range) = composition_selection(&tr.view(), comp) else {
            return Ok(());
        };
        apply_char_format_in_selection(tr, range, &op, &resource)?;
        if let Some(range) = composition_selection(&tr.view(), comp) {
            let paint = replacement_paint(&tr.state().projected, range.anchor, range.head)
                .unwrap_or_default();
            tr.update_meta(|m| m.composition_paint = Some(paint));
        }
        Ok(())
    })
}

fn route_empty_composition_format(
    editor: &mut Editor,
    op: ModifierOp,
    comp: Composition,
) -> Result<(), EditorError> {
    let pos: Position = {
        let view = editor.state().view();
        let Some(rp) = ResolvedPosition::from_flat(&view, comp.start) else {
            return Ok(());
        };
        (&rp).into()
    };

    let resource = Arc::clone(&editor.resource);
    let sidecar = editor.composition_paint.clone().unwrap_or_default();
    let mut scratch = editor.state().clone();
    scratch.composition = None;
    scratch.selection = Some(Selection::collapsed(pos));
    scratch.pending_modifiers = sidecar
        .iter()
        .map(|m| PendingModifier::Set {
            modifier: m.clone(),
        })
        .collect();

    let new_paint: Vec<Modifier> = {
        let resource = resource.lock().unwrap();
        let mut scratch_tr = Transaction::new(&scratch);
        run_collapsed_format(&mut scratch_tr, &op, &resource)?;
        scratch_tr
            .state()
            .pending_modifiers
            .iter()
            .filter_map(|pm| match pm {
                PendingModifier::Set { modifier } => Some(modifier.clone()),
                PendingModifier::Unset { .. } => None,
            })
            .collect()
    };

    editor.transact(|tr| {
        tr.update_meta(|m| m.composition_paint = Some(new_paint.clone()));
        Ok(())
    })
}

fn handle_modifier_during_composition(
    editor: &mut Editor,
    op: ModifierOp,
    comp: Composition,
) -> Result<(), EditorError> {
    if comp.start == comp.end {
        route_empty_composition_format(editor, op, comp)
    } else {
        route_nonempty_composition_format(editor, op, comp)
    }
}

pub fn handle_modifier_op(editor: &mut Editor, op: ModifierOp) -> Result<(), EditorError> {
    if let Some(comp) = editor.state().composition
        && is_composition_routable(&op)
    {
        return handle_modifier_during_composition(editor, op, comp);
    }

    match op {
        ModifierOp::Toggle {
            modifier_type: ModifierType::Bold,
        } => {
            let resource = Arc::clone(&editor.resource);
            let resource = resource.lock().unwrap();
            editor.transact(|tr| {
                commands::toggle_bold(tr, &resource)?;
                Ok(())
            })
        }
        ModifierOp::Toggle { modifier_type } => {
            // toggle_modifier records a span op even when the selected range has
            // no applicable target (e.g. a fold title that suppresses the
            // modifier). That op produces no observable change but still pushes a
            // history entry, which would make editor.can (observable-state based)
            // disagree with apply. Discard the transaction when there is nothing
            // to observe.
            editor.transact_observable(|tr| {
                commands::toggle_modifier(tr, modifier_type)?;
                Ok(())
            })
        }
        ModifierOp::Set {
            modifier: editor_model::Modifier::FontFamily { value },
        } => {
            let resource = Arc::clone(&editor.resource);
            let resource = resource.lock().unwrap();
            editor.transact(|tr| {
                commands::set_font_family(tr, value, &resource)?;
                Ok(())
            })
        }
        ModifierOp::Set { modifier } => editor.transact(|tr| {
            commands::set_modifier(tr, modifier)?;
            Ok(())
        }),
        ModifierOp::SetOnNode { id, modifier } => editor.transact(|tr| {
            commands::set_node_modifier(tr, id, modifier)?;
            Ok(())
        }),
        ModifierOp::Edit {
            modifier_type,
            modifier,
        } => editor.transact(|tr| {
            commands::edit_modifier(tr, modifier_type, modifier)?;
            Ok(())
        }),
        ModifierOp::ClearAll => editor.transact(|tr| {
            commands::clear_all_modifiers(tr)?;
            Ok(())
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use editor_macros::state;
    use editor_resource::{FontFamily, FontFamilySource, FontWeight, Resource};
    use editor_state::assert_state_eq;

    use super::*;
    use crate::event::EditorEvent;
    use crate::state_field::StateField;
    use crate::test_utils::assert_probe_predicts_apply;

    fn make_resource(families: impl IntoIterator<Item = (&'static str, Vec<u16>)>) -> Resource {
        let mut resource = Resource::new_test();
        resource.set_fonts(
            families
                .into_iter()
                .map(|(name, weights)| FontFamily {
                    name: name.to_string(),
                    source: FontFamilySource::Default,
                    weights: weights
                        .into_iter()
                        .map(|value| FontWeight {
                            value,
                            hash: format!("{name}-{value}"),
                        })
                        .collect(),
                })
                .collect(),
        );
        resource
    }

    #[test]
    fn probe_toggle_bold_with_textblock() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1)
        };
        assert_probe_predicts_apply(
            state,
            Message::Modifier {
                op: ModifierOp::Toggle {
                    modifier_type: ModifierType::Bold,
                },
            },
        );
    }

    // The Toggle path must not record a history entry (nor undoable state) when
    // the command produces no observable change — e.g. a selection entirely
    // inside a fold title, which suppresses inline modifiers. `editor.can`
    // (observable-state probe) must keep agreeing with apply.
    #[test]
    fn toggle_with_no_applicable_target_records_no_history() {
        let (state, ..) = state! {
            doc { root { fold { ft1: fold_title { text("title") } fold_content { paragraph { text("body") } } } } }
            selection: (ft1, 0) -> (ft1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Italic,
            },
        });
        assert_eq!(
            editor.history_undos_len(),
            0,
            "no-op toggle must not push an undo entry"
        );
    }

    #[test]
    fn probe_toggle_italic_with_no_applicable_target() {
        let (state, ..) = state! {
            doc { root { fold { ft1: fold_title { text("title") } fold_content { paragraph { text("body") } } } } }
            selection: (ft1, 0) -> (ft1, 5)
        };
        assert_probe_predicts_apply(
            state,
            Message::Modifier {
                op: ModifierOp::Toggle {
                    modifier_type: ModifierType::Italic,
                },
            },
        );
    }

    #[test]
    fn probe_set_modifier_same_value_noop() {
        let (state, ..) = state! {
            doc { root [font_size(1600)] { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        assert_probe_predicts_apply(
            state,
            Message::Modifier {
                op: ModifierOp::Set {
                    modifier: editor_model::Modifier::FontSize { value: 1600 },
                },
            },
        );
    }

    #[test]
    fn probe_clear_all_empty_pending() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        assert_probe_predicts_apply(
            state,
            Message::Modifier {
                op: ModifierOp::ClearAll,
            },
        );
    }

    #[test]
    fn clear_all_collapsed_unsets_effective_inline() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") [italic] } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Modifier {
            op: ModifierOp::ClearAll,
        });
        assert_eq!(
            editor.state().pending_modifiers.as_slice(),
            &[editor_state::PendingModifier::Unset {
                ty: ModifierType::Italic
            }]
        );
    }

    #[test]
    fn clear_all_range_removes_inline_from_doc() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello") [italic] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Modifier {
            op: ModifierOp::ClearAll,
        });
        let view = editor.state().view();
        let para = view.node(p1).unwrap();
        assert!(
            !para.inline().iter().any(|item| item
                .effective
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Italic))),
            "italic must be cleared from all leaves"
        );
    }

    #[test]
    fn toggle_italic_via_message() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Italic,
            },
        });
        assert_eq!(
            editor.state().pending_modifiers.as_slice(),
            &[editor_state::PendingModifier::Set {
                modifier: editor_model::Modifier::Italic
            }]
        );
    }

    #[test]
    fn set_font_size_via_message() {
        let (state, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("hello") }
                }
            }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Modifier {
            op: ModifierOp::Set {
                modifier: editor_model::Modifier::FontSize { value: 2400 },
            },
        });
        assert_eq!(
            editor.state().pending_modifiers.as_slice(),
            &[editor_state::PendingModifier::Set {
                modifier: editor_model::Modifier::FontSize { value: 2400 }
            }]
        );
    }

    #[test]
    fn set_font_family_via_message_normalizes_unavailable_weight() {
        let resource = Arc::new(Mutex::new(make_resource([
            ("Source", vec![400, 700]),
            ("LightFont", vec![100, 300]),
        ])));
        let (state, ..) = state! {
            doc {
                root [font_family("Source".to_string()), font_weight(400)] {
                    p1: paragraph { text("hello") [font_weight(700)] }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);

        editor.apply(Message::Modifier {
            op: ModifierOp::Set {
                modifier: editor_model::Modifier::FontFamily {
                    value: "LightFont".to_string(),
                },
            },
        });

        let (expected, ..) = state! {
            doc {
                root [font_family("Source".to_string()), font_weight(400)] {
                    p1: paragraph carry([font_family("LightFont".to_string()), font_weight(300)]) {
                        text("hello") [font_weight(300), font_family("LightFont".to_string()), bold]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn set_on_node_root_sets_document_default_font_family() {
        let (state, ..) = state! {
            doc {
                root [font_family("Pretendard".to_string()), font_weight(400)] {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);

        let events = editor.apply(Message::Modifier {
            op: ModifierOp::SetOnNode {
                id: editor_crdt::Dot::ROOT,
                modifier: editor_model::Modifier::FontFamily {
                    value: "Paperlogy".to_string(),
                },
            },
        });

        assert!(events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::Modifiers)
        )));
        let view = editor.state().view();
        let root = view.node(editor_crdt::Dot::ROOT).unwrap();
        assert!(matches!(
            root.block_modifier(editor_model::ModifierType::FontFamily),
            Some(editor_model::Modifier::FontFamily { value }) if value == "Paperlogy"
        ));
    }

    #[test]
    fn toggle_italic_on_block_unit_selection_via_message() {
        let (state, ..) = state! {
            doc { r1: root { p1: paragraph { text("안녕하세요") } } }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Italic,
            },
        });

        let (expected, ..) = state! {
            doc { r1: root { p1: paragraph carry([italic]) { text("안녕하세요") [italic] } } }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn toggle_italic_skips_fold_title_and_applies_to_paragraph_via_message() {
        let (state, ..) = state! {
            doc { root {
                fold {
                    ft1: fold_title { text("Title") }
                    fold_content { p1: paragraph { text("Body") } }
                }
            } }
            selection: (ft1, 0) -> (p1, 4)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Italic,
            },
        });

        let (expected, ..) = state! {
            doc { root {
                fold {
                    ft1: fold_title { text("Title") }
                    fold_content { p1: paragraph carry([italic]) { text("Body") [italic] } }
                }
            } }
            selection: (ft1, 0) -> (p1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn toggle_underline_during_composition_applies_to_and_keeps_composition_text() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("하") } } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "가".into() }],
        });

        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Underline,
            },
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "각".into() }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::CommitAsIs],
        });

        let (expected, ..) = state! {
            doc { root { p1: paragraph carry([underline]) {
                text("하")
                text("각") [underline]
            } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn toggle_bold_during_empty_composition_seeds_first_composed_char() {
        let resource = Arc::new(Mutex::new(make_resource([("Pretendard", vec![400, 700])])));
        let (state, p1) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph { text("하") }
                }
            }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);

        let flat = {
            let view = editor.state().view();
            editor
                .state()
                .selection
                .unwrap()
                .head
                .resolve(&view)
                .unwrap()
                .to_flat()
        };
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition {
                start: flat,
                end: flat,
            }],
        });
        assert_eq!(
            editor.state().composition,
            Some(editor_state::Composition {
                start: flat,
                end: flat
            })
        );

        let before_text = {
            let view = editor.state().view();
            editor_state::flat_text(&view, 0..editor_state::flat_size(&view))
        };
        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Bold,
            },
        });
        let after_text = {
            let view = editor.state().view();
            editor_state::flat_text(&view, 0..editor_state::flat_size(&view))
        };
        assert_eq!(
            before_text, after_text,
            "empty-composition format must not change the document"
        );

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "가".into() }],
        });

        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("하")
                        text("가") [font_weight(700)]
                    }
                }
            }
            selection: (p1, 2)
        };
        let _ = p1;
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn empty_composition_format_is_observable_and_reflected() {
        let resource = Arc::new(Mutex::new(make_resource([("Pretendard", vec![400, 700])])));
        let (state, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph { text("하") }
                }
            }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);

        let flat = {
            let view = editor.state().view();
            editor
                .state()
                .selection
                .unwrap()
                .head
                .resolve(&view)
                .unwrap()
                .to_flat()
        };
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition {
                start: flat,
                end: flat,
            }],
        });

        let can = editor
            .can(Message::Modifier {
                op: ModifierOp::Toggle {
                    modifier_type: ModifierType::Bold,
                },
            })
            .unwrap();
        assert!(can, "empty-composition format command is enabled via can()");

        let events = editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Bold,
            },
        });
        assert!(
            events.iter().any(|e| matches!(
                e,
                EditorEvent::StateChanged { fields } if fields.contains(&StateField::Modifiers)
            )),
            "the sidecar update fires a Modifiers state change"
        );

        let ms = editor.modifier_state().expect("modifier state available");
        assert!(
            matches!(ms.effective_bold, editor_common::Tri::Uniform { .. }),
            "modifier_state reflects the confirmed composition paint (bold)"
        );
    }

    #[test]
    fn composition_format_commands_preserve_selection() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("하") } } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "가".into() }],
        });
        let sel_before = editor.state().selection;
        assert!(sel_before.is_some());

        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Underline,
            },
        });
        assert_eq!(
            editor.state().selection,
            sel_before,
            "toggle during composition keeps the caret"
        );

        editor.apply(Message::Modifier {
            op: ModifierOp::Edit {
                modifier_type: ModifierType::BackgroundColor,
                modifier: Some(editor_model::Modifier::BackgroundColor {
                    value: "red".to_string(),
                }),
            },
        });
        assert_eq!(
            editor.state().selection,
            sel_before,
            "background-color edit during composition keeps the caret"
        );

        editor.apply(Message::Modifier {
            op: ModifierOp::ClearAll,
        });
        assert_eq!(
            editor.state().selection,
            sel_before,
            "clear-all during composition keeps the caret"
        );

        assert!(
            editor.state().composition.is_some(),
            "composition stays active across format commands"
        );
    }
}
