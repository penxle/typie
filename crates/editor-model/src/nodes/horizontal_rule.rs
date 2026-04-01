use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HorizontalRuleNode {
    #[serde(default)]
    pub variant: HorizontalRuleVariant,
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
