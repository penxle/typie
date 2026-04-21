use editor_commands::{self as commands};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_clipboard_op(editor: &mut Editor, op: ClipboardOp) -> Result<(), EditorError> {
    editor.transact(|tr| {
        match op {
            ClipboardOp::Paste { text, html } => {
                if let Some(_) = html {
                    // not yet implemented
                } else {
                    commands::chain!(
                        tr,
                        commands::optional!(commands::delete_selection()),
                        commands::insert_text(&text),
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    })
}
