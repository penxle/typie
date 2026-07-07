use editor_crdt::LwwReg;
use editor_macros::{NodeAttr, ffi};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct BlockquoteNode {
    #[plain(serde(default))]
    pub variant: LwwReg<BlockquoteVariant>,
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
