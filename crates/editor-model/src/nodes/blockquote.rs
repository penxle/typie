use editor_crdt::LwwReg;
use editor_macros::{NodeAttr, ffi};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct BlockquoteNode {
    #[plain(serde(default))]
    pub variant: LwwReg<BlockquoteVariant>,
}

#[ffi]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, editor_macros::Wire,
)]
#[serde(rename_all = "snake_case")]
pub enum BlockquoteVariant {
    #[default]
    #[wire(n(0))]
    LeftLine,
    #[wire(n(1))]
    LeftQuote,
    #[wire(n(2))]
    MessageSent,
    #[wire(n(3))]
    MessageReceived,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blockquote_variant_wire_round_trip() {
        use editor_crdt::wire::{DecCtx, EncCtx, Wire};
        let ec = EncCtx::from_table(&[], vec![]);
        let dc = DecCtx {
            actor_table: vec![],
            baselines: vec![],
        };
        let cases = [
            BlockquoteVariant::LeftLine,
            BlockquoteVariant::LeftQuote,
            BlockquoteVariant::MessageSent,
            BlockquoteVariant::MessageReceived,
        ];
        for v in cases {
            let mut buf = Vec::new();
            <BlockquoteVariant as Wire>::encode(&v, &ec, &mut buf).unwrap();
            let mut slice = &buf[..];
            let got = <BlockquoteVariant as Wire>::decode(&dc, &mut slice).unwrap();
            assert_eq!(got, v);
        }
    }
}
