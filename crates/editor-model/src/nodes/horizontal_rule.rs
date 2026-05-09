use editor_crdt::LwwReg;
use editor_macros::{NodeAttr, ffi};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct HorizontalRuleNode {
    #[plain(serde(default))]
    pub variant: LwwReg<HorizontalRuleVariant>,
}

#[ffi]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Encode, Decode,
)]
#[cbor(index_only)]
#[serde(rename_all = "snake_case")]
pub enum HorizontalRuleVariant {
    #[default]
    #[n(0)]
    Line,
    #[n(1)]
    DashedLine,
    #[n(2)]
    CircleLine,
    #[n(3)]
    DiamondLine,
    #[n(4)]
    Circle,
    #[n(5)]
    Diamond,
    #[n(6)]
    ThreeCircles,
    #[n(7)]
    ThreeDiamonds,
    #[n(8)]
    Zigzag,
}
