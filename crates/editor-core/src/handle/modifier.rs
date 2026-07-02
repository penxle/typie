use editor_commands::{self as commands};
use editor_model::ModifierType;
use std::sync::Arc;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_modifier_op(editor: &mut Editor, op: ModifierOp) -> Result<(), EditorError> {
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
            // disagree with apply. Skip the transaction when there is nothing to
            // observe.
            let mut probe = editor_transaction::Transaction::new(&editor.state);
            commands::toggle_modifier(&mut probe, modifier_type)?;
            if !editor_state::state_observably_changed(&editor.state, probe.state()) {
                return Ok(());
            }
            editor.transact(|tr| {
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
                            chunks: vec![vec![0x0000, 0xFFFF]],
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
                    p1: paragraph {
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
                    fold_content { p1: paragraph { text("Body") [italic] } }
                }
            } }
            selection: (ft1, 0) -> (p1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}
