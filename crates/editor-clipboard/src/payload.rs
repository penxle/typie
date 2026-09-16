use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardPayload {
    pub html: String,
    pub text: String,
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
    fn payload_holds_html_and_text() {
        let p = ClipboardPayload {
            html: "<p>hi</p>".into(),
            text: "hi".into(),
        };
        assert_eq!(p.html, "<p>hi</p>");
        assert_eq!(p.text, "hi");
    }
}
