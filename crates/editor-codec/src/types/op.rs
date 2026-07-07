use editor_codec_macros::Durable;
use editor_crdt::Dot;

use crate::framing::{UnknownPayload, UnknownTail};
use crate::types::anchor::DurableAnchor;
use crate::types::attr::DurableAttr;
use crate::types::item::DurableItem;
use crate::types::modifier::{DurableModifier, DurableModifierKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Durable)]
#[durable(frozen)]
pub struct DurableAliasRun {
    pub old_start: Dot,
    pub len: u64,
    pub new_start: Dot,
}

#[derive(Debug, Clone, PartialEq, Eq, Durable)]
#[durable(open)]
pub enum DurableOp {
    #[durable(n(0))]
    #[durable(frozen)]
    SeqIns { pos: u64, item: DurableItem },
    #[durable(n(1))]
    #[durable(frozen)]
    SeqDel { pos: u64, len: u64 },
    #[durable(n(2))]
    #[durable(frozen)]
    SeqUndel { del: Dot },
    #[durable(n(3))]
    AddSpan {
        start: DurableAnchor,
        end: DurableAnchor,
        modifier: DurableModifier,
        tail: UnknownTail,
    },
    #[durable(n(4))]
    RemoveSpan {
        start: DurableAnchor,
        end: DurableAnchor,
        kind: DurableModifierKind,
        tail: UnknownTail,
    },
    #[durable(n(5))]
    SetBlockModifier {
        target: Dot,
        modifier: DurableModifier,
        tail: UnknownTail,
    },
    #[durable(n(6))]
    ClearBlockModifier {
        target: Dot,
        kind: DurableModifierKind,
        tail: UnknownTail,
    },
    #[durable(n(7))]
    SetNodeAttr {
        target: Dot,
        attr: DurableAttr,
        tail: UnknownTail,
    },
    #[durable(n(8))]
    SetNodeCarry {
        target: Dot,
        modifier: DurableModifier,
        tail: UnknownTail,
    },
    #[durable(n(9))]
    ClearNodeCarry {
        target: Dot,
        kind: DurableModifierKind,
        tail: UnknownTail,
    },
    #[durable(n(10))]
    AliasDots {
        pairs: Vec<DurableAliasRun>,
        tail: UnknownTail,
    },
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

impl DurableOp {
    pub fn contains_ctx_unknown(&self) -> bool {
        match self {
            DurableOp::SeqIns { item, .. } => item.contains_ctx_unknown(),
            DurableOp::SeqDel { .. } | DurableOp::SeqUndel { .. } => false,
            DurableOp::AddSpan { modifier, tail, .. } => {
                modifier.contains_ctx_unknown() || !tail.0.is_empty()
            }
            DurableOp::RemoveSpan { kind, tail, .. } => {
                kind.contains_ctx_unknown() || !tail.0.is_empty()
            }
            DurableOp::SetBlockModifier { modifier, tail, .. }
            | DurableOp::SetNodeCarry { modifier, tail, .. } => {
                modifier.contains_ctx_unknown() || !tail.0.is_empty()
            }
            DurableOp::ClearBlockModifier { kind, tail, .. }
            | DurableOp::ClearNodeCarry { kind, tail, .. } => {
                kind.contains_ctx_unknown() || !tail.0.is_empty()
            }
            DurableOp::SetNodeAttr { tail, .. } => !tail.0.is_empty(),
            DurableOp::AliasDots { tail, .. } => !tail.0.is_empty(),
            DurableOp::Unknown(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctx::{DecCtx, EncCtx};
    use crate::durable::Durable;

    fn round_trip(op: &DurableOp) -> DurableOp {
        let enc = EncCtx::from_parts(&[7], vec![0]).unwrap();
        let dec = DecCtx {
            actors: vec![7],
            baselines: vec![0],
        };
        let mut bytes = Vec::new();
        op.encode(&enc, &mut bytes).unwrap();
        let mut slice = &bytes[..];
        let decoded = DurableOp::decode(&dec, &mut slice).unwrap();
        assert!(slice.is_empty());
        decoded
    }

    #[test]
    fn representative_ops_round_trip() {
        use crate::framing::UnknownTail;
        use crate::types::item::{DurableItem, DurableNodeType};

        let ops = vec![
            DurableOp::SeqIns {
                pos: 3,
                item: DurableItem::Char('한'),
            },
            DurableOp::SeqIns {
                pos: 0,
                item: DurableItem::Block {
                    node_type: DurableNodeType::Callout,
                    parents: vec![Dot::new(7, 1)],
                    init: vec![crate::types::attr::DurableAttr::CalloutVariant(
                        crate::types::values::DurableCalloutVariant::Warning,
                    )],
                    tail: UnknownTail(vec![]),
                },
            },
            DurableOp::SeqDel { pos: 5, len: 2 },
            DurableOp::SeqUndel {
                del: Dot::new(7, 4),
            },
            DurableOp::SetNodeAttr {
                target: Dot::new(7, 2),
                attr: crate::types::attr::DurableAttr::ImageProportion(80),
                tail: UnknownTail(vec![]),
            },
        ];
        for op in &ops {
            assert_eq!(&round_trip(op), op);
        }
    }
}
