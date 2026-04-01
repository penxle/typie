use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::StateField;

#[ffi]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EditorEvent {
    StateChanged { fields: Vec<StateField> },
    DocumentChanged,
    RenderInvalidated,
    FontMissing { family: String, weight: u16 },
    CursorExitedDocumentStart,
}
