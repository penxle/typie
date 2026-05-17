use editor_crdt::LwwReg;
use editor_macros::{NodeAttr, ffi};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct CalloutNode {
    #[plain(serde(default))]
    pub variant: LwwReg<CalloutVariant>,
}

#[ffi]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, editor_macros::Wire,
)]
#[serde(rename_all = "snake_case")]
pub enum CalloutVariant {
    #[default]
    #[wire(n(0))]
    Info,
    #[wire(n(1))]
    Success,
    #[wire(n(2))]
    Warning,
    #[wire(n(3))]
    Danger,
}

impl CalloutVariant {
    pub fn next(self) -> Self {
        match self {
            Self::Info => Self::Success,
            Self::Success => Self::Warning,
            Self::Warning => Self::Danger,
            Self::Danger => Self::Info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callout_variant_next_cycles() {
        assert_eq!(CalloutVariant::Info.next(), CalloutVariant::Success);
        assert_eq!(CalloutVariant::Success.next(), CalloutVariant::Warning);
        assert_eq!(CalloutVariant::Warning.next(), CalloutVariant::Danger);
        assert_eq!(CalloutVariant::Danger.next(), CalloutVariant::Info);
    }

    #[test]
    fn callout_variant_wire_round_trip() {
        use editor_crdt::wire::{DecCtx, EncCtx, Wire};
        let ec = EncCtx::from_table(&[], vec![]);
        let dc = DecCtx {
            actor_table: vec![],
            baselines: vec![],
        };
        let cases = [
            CalloutVariant::Info,
            CalloutVariant::Success,
            CalloutVariant::Warning,
            CalloutVariant::Danger,
        ];
        for v in cases {
            let mut buf = Vec::new();
            <CalloutVariant as Wire>::encode(&v, &ec, &mut buf).unwrap();
            let mut slice = &buf[..];
            let got = <CalloutVariant as Wire>::decode(&dc, &mut slice).unwrap();
            assert_eq!(got, v);
        }
    }
}
