use editor_macros::ffi;
use editor_model::Modifier;
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MergeKind {
    #[default]
    Isolated,
    Typing,
}

#[ffi]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TransactionMeta {
    pub history: HistoryMeta,
    #[serde(default)]
    pub composition_paint: Option<Vec<Modifier>>,
    #[serde(skip)]
    pub merge: MergeKind,
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
            composition_paint: None,
            merge: MergeKind::Isolated,
        };
        let json = serde_json::to_string(&meta).unwrap();
        let back: TransactionMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, back);
    }

    #[test]
    fn composition_paint_roundtrip() {
        let meta = TransactionMeta {
            history: HistoryMeta::default(),
            composition_paint: Some(vec![Modifier::Bold, Modifier::Italic]),
            merge: MergeKind::Isolated,
        };
        let json = serde_json::to_string(&meta).unwrap();
        let back: TransactionMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, back);
    }

    #[test]
    fn legacy_json_without_composition_paint_deserializes() {
        let back: TransactionMeta =
            serde_json::from_str(r#"{"history":{"type":"record"}}"#).unwrap();
        assert_eq!(back, TransactionMeta::default());
        assert!(back.composition_paint.is_none());
    }
}
