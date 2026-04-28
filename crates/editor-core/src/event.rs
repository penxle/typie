use editor_macros::ffi;
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
        steps: Vec<Step>,
        meta: TransactionMeta,
    },
}

#[cfg(test)]
mod tests {
    use editor_model::NodeId;

    use super::*;

    #[test]
    fn transaction_committed_serializes() {
        let event = EditorEvent::TransactionCommitted {
            steps: vec![Step::InsertText {
                node_id: NodeId::new(),
                offset: 0,
                text: "hi".into(),
            }],
            meta: TransactionMeta::default(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"transaction_committed\""));
        assert!(json.contains("\"steps\""));
        let decoded: EditorEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, event);
    }
}
