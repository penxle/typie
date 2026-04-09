use editor_commands::{self as commands};
use editor_model::ModifierType;
use std::sync::Arc;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_formatting_op(editor: &mut Editor, op: FormattingOp) -> Result<(), EditorError> {
    match op {
        FormattingOp::ToggleModifier {
            modifier_type: ModifierType::Bold,
        } => {
            let resource = Arc::clone(&editor.resource);
            let resource = resource.lock().unwrap();
            editor.transact(|tr| {
                commands::toggle_bold(tr, &resource)?;
                Ok(())
            })
        }
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
        editor.apply(Message::Formatting {
            op: FormattingOp::ClearModifiers,
        });
        assert_eq!(editor.state().selection, state.selection);
    }
}
