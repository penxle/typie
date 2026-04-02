use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::StateField;

#[ffi]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum EditorEvent {
    StateChanged {
        fields: Vec<StateField>,
    },
    DocumentChanged,
    RenderInvalidated,
    FontManifestMissing {
        family: String,
        weight: u16,
    },
    FontDataMissing {
        family: String,
        weight: u16,
        required: Vec<FontData>,
        prefetch: Vec<FontData>,
    },
    CursorExitedDocumentStart,
}

#[ffi]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FontData {
    Base,
    Chunk(u16),
}
