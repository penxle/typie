use editor_crdt::LwwReg;
use editor_macros::{NodeAttr, ffi};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct BlockquoteNode {
    #[plain(serde(default))]
    pub variant: LwwReg<BlockquoteVariant>,
}

#[ffi]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Encode, Decode,
)]
#[cbor(index_only)]
#[serde(rename_all = "snake_case")]
pub enum BlockquoteVariant {
    #[default]
    #[n(0)]
    LeftLine,
    #[n(1)]
    LeftQuote,
    #[n(2)]
    MessageSent,
    #[n(3)]
    MessageReceived,
}
