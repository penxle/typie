use editor_crdt::LwwReg;
use editor_macros::{NodeAttr, ffi};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct HorizontalRuleNode {
    #[plain(serde(default))]
    pub variant: LwwReg<HorizontalRuleVariant>,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HorizontalRuleVariant {
    #[default]
    Line,
    DashedLine,
    CircleLine,
    DiamondLine,
    Circle,
    Diamond,
    ThreeCircles,
    ThreeDiamonds,
    Zigzag,
}
