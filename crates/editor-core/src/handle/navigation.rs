use editor_state::Selection;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_navigation_intent(
    editor: &mut Editor,
    nav: NavigationIntent,
) -> Result<(), EditorError> {
    match nav {
        NavigationIntent::Move { movement, extend } => {
            let selection = editor.state.selection;
            if let Some(new_selection) = {
                let resource = editor.resource.lock().unwrap();
                editor.view.resolve_movement(
                    &selection.head,
                    &movement,
                    &editor.state.doc,
                    resource.segmenters.as_ref(),
                )
            } {
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
