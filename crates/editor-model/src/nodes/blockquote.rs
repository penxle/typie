use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BlockquoteNode {
    #[serde(default)]
    pub variant: BlockquoteVariant,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockquoteVariant {
    #[default]
    LeftLine,
    LeftQuote,
    MessageSent,
    MessageReceived,
}
