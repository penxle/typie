use crate::drag_ghost::DragGhost;
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardPayload {
    pub html: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drag_ghost: Option<DragGhost>,
}

/// Already-resolved asset metadata. Copy never fetches or encodes asset bytes.
#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardAsset {
    pub id: String,
    pub url: String,
    pub label: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_without_ghost_preserves_clipboard_wire_format() {
        let p = ClipboardPayload {
            html: "<p>hi</p>".into(),
            text: "hi".into(),
            drag_ghost: None,
        };
        let json = serde_json::json!({ "html": "<p>hi</p>", "text": "hi" });
        assert_eq!(serde_json::to_value(&p).unwrap(), json);
        assert_eq!(serde_json::from_value::<ClipboardPayload>(json).unwrap(), p);
    }
}
