use editor_crdt::LwwReg;
use editor_macros::{NodeAttr, ffi};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct HorizontalRuleNode {
    #[plain(serde(default))]
    pub variant: LwwReg<HorizontalRuleVariant>,
}

#[ffi]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, editor_macros::Wire,
)]
#[serde(rename_all = "snake_case")]
pub enum HorizontalRuleVariant {
    #[default]
    #[wire(n(0))]
    Line,
    #[wire(n(1))]
    DashedLine,
    #[wire(n(2))]
    CircleLine,
    #[wire(n(3))]
    DiamondLine,
    #[wire(n(4))]
    Circle,
    #[wire(n(5))]
    Diamond,
    #[wire(n(6))]
    ThreeCircles,
    #[wire(n(7))]
    ThreeDiamonds,
    #[wire(n(8))]
    Zigzag,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_rule_variant_wire_round_trip() {
        use editor_crdt::wire::{DecCtx, EncCtx, Wire};
        let ec = EncCtx::from_table(&[], vec![]);
        let dc = DecCtx {
            actor_table: vec![],
            baselines: vec![],
        };
        let cases = [
            HorizontalRuleVariant::Line,
            HorizontalRuleVariant::DashedLine,
            HorizontalRuleVariant::CircleLine,
            HorizontalRuleVariant::DiamondLine,
            HorizontalRuleVariant::Circle,
            HorizontalRuleVariant::Diamond,
            HorizontalRuleVariant::ThreeCircles,
            HorizontalRuleVariant::ThreeDiamonds,
            HorizontalRuleVariant::Zigzag,
        ];
        for v in cases {
            let mut buf = Vec::new();
            <HorizontalRuleVariant as Wire>::encode(&v, &ec, &mut buf).unwrap();
            let mut slice = &buf[..];
            let got = <HorizontalRuleVariant as Wire>::decode(&dc, &mut slice).unwrap();
            assert_eq!(got, v);
        }
    }
}
