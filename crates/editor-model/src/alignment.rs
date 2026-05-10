use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, editor_macros::Wire,
)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    #[default]
    #[wire(n(0))]
    Left,
    #[wire(n(1))]
    Center,
    #[wire(n(2))]
    Right,
    #[wire(n(3))]
    Justify,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_wire_round_trip_all_variants() {
        use editor_crdt::wire::{DecCtx, EncCtx, Wire};
        let ec = EncCtx::from_table(&[], vec![]);
        let dc = DecCtx {
            actor_table: vec![],
            baselines: vec![],
        };
        let cases = [
            Alignment::Left,
            Alignment::Center,
            Alignment::Right,
            Alignment::Justify,
        ];
        for v in cases {
            let mut buf = Vec::new();
            <Alignment as Wire>::encode(&v, &ec, &mut buf).unwrap();
            let mut slice = &buf[..];
            let got = <Alignment as Wire>::decode(&dc, &mut slice).unwrap();
            assert_eq!(got, v);
        }
    }
}
