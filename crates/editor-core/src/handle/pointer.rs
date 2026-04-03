use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_pointer_event(editor: &mut Editor, event: PointerEvent) -> Result<(), EditorError> {
    match event {
        PointerEvent::Down { page, x, y, .. } => {
            if let Some(new_selection) = { editor.view.hit_test(page, x, y, &editor.state.doc) } {
                editor.transact(|tr| {
                    tr.set_selection(new_selection)?;
                    Ok(())
                })?;
            }
        }
    }
    Ok(())
}
