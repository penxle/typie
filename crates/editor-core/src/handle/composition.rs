use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_composition_intent(
    _editor: &mut Editor,
    intent: CompositionIntent,
) -> Result<(), EditorError> {
    match intent {
        CompositionIntent::Update { .. } => {}
        CompositionIntent::SetRegion { .. } => {}
        CompositionIntent::Commit { .. } => {}
        CompositionIntent::CommitAsIs => {}
        CompositionIntent::Cancel => {}
    }
    Ok(())
}
