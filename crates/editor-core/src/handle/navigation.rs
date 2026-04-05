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
            let segmenters = editor.resource.lock().unwrap().segmenters.clone();
            if let Some(new_selection) = editor.view.resolve_movement(
                &selection.head,
                &movement,
                &editor.state.doc,
                segmenters.as_deref(),
            ) {
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
