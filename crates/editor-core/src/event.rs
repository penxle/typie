use editor_macros::ffi;
use editor_model::DerivedObject;
use editor_transaction::{Step, TransactionMeta};
use serde::{Deserialize, Serialize};

use crate::state_field::StateField;

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FontData {
    Base,
    Chunk { id: u16 },
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitPayload {
    pub root_object_hash: String,
    pub new_objects: Vec<DerivedObject>,
    pub steps: Vec<Step>,
    pub meta: TransactionMeta,
    pub committed_at: i64,
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
    TransactionCommitted {
        #[serde(rename = "commitPayload")]
        commit_payload: CommitPayload,
    },
}

#[cfg(test)]
mod tests {
    use editor_model::NodeId;

    use super::*;

    #[test]
    fn transaction_committed_serializes() {
        let event = EditorEvent::TransactionCommitted {
            commit_payload: CommitPayload {
                root_object_hash: "0".repeat(32),
                new_objects: vec![],
                steps: vec![Step::InsertText {
                    node_id: NodeId::new(),
                    offset: 0,
                    text: "hi".into(),
                }],
                meta: TransactionMeta::default(),
                committed_at: 0,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"transaction_committed\""));
        assert!(json.contains("\"commitPayload\""));
        assert!(json.contains("\"rootObjectHash\""));
        assert!(json.contains("\"newObjects\""));
        let decoded: EditorEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, event);
    }
}
