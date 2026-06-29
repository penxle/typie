use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HistoryTag {
    AutoReplacement,
    PasteHtml {
        plain_text: String,
        start: Option<usize>,
    },
}
