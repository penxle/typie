use editor_crdt::Dot;
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::state_field::StateField;

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FontData {
    Base,
    Chunk { id: u16 },
    Manifest,
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackedRangeReplaceOutcome {
    Replaced,
    UnknownId,
    Invalid,
    TextMismatch,
    InvalidReplacement,
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProseRangeInstallOutcome {
    Applied,
    TextMismatch,
    InvalidRanges { indices: Vec<u32> },
    InvalidRequest,
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EditorEvent {
    StateChanged {
        fields: Vec<StateField>,
    },
    RenderInvalidated,
    FontDataMissing {
        family: String,
        weight: u16,
        required: Vec<FontData>,
        prefetch: Vec<FontData>,
    },
    CursorExitedDocumentStart,
    ImeResyncRequired,
    TrackedRangeReplaceResult {
        id: String,
        outcome: TrackedRangeReplaceOutcome,
    },
    /// Text-sensitive tracked ranges the editor removed this tick because the
    /// text they covered no longer equals their install-time capture (or no
    /// longer resolves at all). Hosts drop the corresponding UI entries
    /// instead of re-reading and re-comparing every range's text themselves.
    TrackedRangesStale {
        ids: Vec<String>,
    },
    ProseRangeInstallResult {
        outcome: ProseRangeInstallOutcome,
    },
    AttachmentPlaceholdersInserted {
        request_id: String,
        node_ids: Vec<Dot>,
    },
}
