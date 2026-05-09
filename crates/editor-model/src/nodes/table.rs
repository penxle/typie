use editor_crdt::LwwReg;
use editor_macros::{NodeAttr, ffi};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct TableNode {
    #[plain(serde(default))]
    pub border_style: LwwReg<TableBorderStyle>,
    #[node_attr(default = "100u32")]
    #[plain(ffi(default = "100"), serde(default = "default_proportion"))]
    pub proportion: LwwReg<u32>,
}

fn default_proportion() -> u32 {
    100
}

#[ffi]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Encode, Decode,
)]
#[cbor(index_only)]
#[serde(rename_all = "snake_case")]
pub enum TableBorderStyle {
    #[default]
    #[n(0)]
    Solid,
    #[n(1)]
    Dashed,
    #[n(2)]
    Dotted,
    #[n(3)]
    None,
}
