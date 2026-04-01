use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Clone, Debug, Default, PartialEq, Hash, Serialize, Deserialize)]
pub struct TextNode {
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::StrExt;

    #[test]
    fn empty_text_node() {
        let t = TextNode::default();
        assert_eq!(t.text.char_count(), 0);
        assert_eq!(t.text, "");
    }

    #[test]
    fn chars_count_counts_chars_not_bytes() {
        let t = TextNode {
            text: "한글".into(),
        };
        assert_eq!(t.text.char_count(), 2);
    }

    #[test]
    fn serde_roundtrip() {
        let t = TextNode {
            text: "Hello".into(),
        };
        let json = serde_json::to_string(&t).unwrap();
        let parsed: TextNode = serde_json::from_str(&json).unwrap();
        assert_eq!(t, parsed);
    }
}
