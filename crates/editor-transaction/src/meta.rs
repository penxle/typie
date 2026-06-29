use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use editor_common::HistoryTag;

#[ffi]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HistoryMeta {
    #[default]
    Record,
    Tagged {
        tag: HistoryTag,
    },
    Skip,
}

#[ffi]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TransactionMeta {
    pub history: HistoryMeta,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_tagged_paste_html_roundtrip() {
        let meta = TransactionMeta {
            history: HistoryMeta::Tagged {
                tag: HistoryTag::PasteHtml {
                    plain_text: "hi".into(),
                    start: None,
                },
            },
        };
        let json = serde_json::to_string(&meta).unwrap();
        let back: TransactionMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, back);
    }
}
