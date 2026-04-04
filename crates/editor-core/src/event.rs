use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::state_field::StateField;

#[ffi]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EditorEvent {
    StateChanged {
        fields: Vec<StateField>,
    },
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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FontData {
    Base,
    Chunk { index: u16 },
}
