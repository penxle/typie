use editor_crdt::LwwReg;
use editor_macros::{NodeAttr, ffi};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct CalloutNode {
    #[plain(serde(default))]
    pub variant: LwwReg<CalloutVariant>,
}

#[ffi]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Encode, Decode,
)]
#[cbor(index_only)]
#[serde(rename_all = "snake_case")]
pub enum CalloutVariant {
    #[default]
    #[n(0)]
    Info,
    #[n(1)]
    Success,
    #[n(2)]
    Warning,
    #[n(3)]
    Danger,
}
