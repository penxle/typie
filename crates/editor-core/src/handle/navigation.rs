use editor_state::Selection;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_navigation_op(editor: &mut Editor, op: NavigationOp) -> Result<(), EditorError> {
    match op {
        NavigationOp::Move { movement, extend } => {
            let selection = editor.state.selection;
            let resource_guard = editor.resource.lock().unwrap();
            let new_selection =
                editor
                    .view
                    .resolve_movement(&selection.head, &movement, &resource_guard);
            drop(resource_guard);
            if let Some(new_selection) = new_selection {
                editor.transact(|tr| {
                    let selection = if extend {
                        Selection::new(selection.anchor, new_selection.head)
                    } else {
                        new_selection
                    };

                    tr.set_selection(selection)?;
                    Ok(())
                })?;
            }
        }
    }
    Ok(())
}
