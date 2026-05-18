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
        ModifierOp::Toggle { modifier_type } => editor.transact(|tr| {
            commands::toggle_modifier(tr, modifier_type)?;
            Ok(())
        }),
        ModifierOp::Set { modifier } => editor.transact(|tr| {
            commands::set_modifier(tr, modifier)?;
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
    use editor_macros::state;
    use editor_state::assert_state_eq;

    use super::*;

    #[test]
    fn clear_all_collapsed_unsets_effective_inline() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") [italic] } } }
            selection: (t1, 2)
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
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello") [italic] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Modifier {
            op: ModifierOp::ClearAll,
        });
        let entry = editor.state().doc.get_entry(t1).unwrap();
        assert!(
            !entry
                .modifiers
                .iter()
                .any(|(_, m)| matches!(m, editor_model::Modifier::Italic))
        );
    }

    #[test]
    fn toggle_italic_via_message() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
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
                    paragraph { t1: text("hello") }
                }
            }
            selection: (t1, 2)
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
    fn toggle_italic_skips_fold_title_and_applies_to_paragraph_via_message() {
        let (state, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("Title") }
                    fold_content { paragraph { t2: text("Body") } }
                }
            } }
            selection: (t1, 0) -> (t2, 4)
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
                    fold_title { t1: text("Title") }
                    fold_content { paragraph { t2: text("Body") [italic] } }
                }
            } }
            selection: (t1, 0) -> (t2, 4)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}
