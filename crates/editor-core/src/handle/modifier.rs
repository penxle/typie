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
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;

    #[test]
    fn unimplemented_is_noop() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state.clone());
        editor.apply(Message::Modifier {
            op: ModifierOp::ClearAll,
        });
        assert_eq!(editor.state().selection, state.selection);
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
}
