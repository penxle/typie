use crate::Dot;
use crate::wire::{DecCtx, EncCtx, Wire, WireError, WireResult, varint};

impl Wire for Dot {
    fn collect(&self, ctx: &mut crate::wire::CollectCtx) {
        ctx.observe(self);
    }

    fn encode(&self, ctx: &EncCtx, out: &mut Vec<u8>) -> WireResult<()> {
        let idx = ctx.actor_idx(self.actor);
        let baseline = ctx.baseline(idx);
        if self.clock < baseline {
            return Err(WireError::BaselineUnderflow {
                actor: self.actor,
                clock: self.clock,
                baseline,
            });
        }
        varint::write_varint(idx as u64, out);
        varint::write_varint(self.clock - baseline, out);
        Ok(())
    }

    fn decode(ctx: &DecCtx, input: &mut &[u8]) -> WireResult<Self> {
        let idx_u64 = varint::read_varint(input)?;
        if idx_u64 > u32::MAX as u64 {
            return Err(WireError::ActorIdxOutOfRange {
                idx: idx_u64,
                table_len: ctx.actor_table.len(),
            });
        }
        let delta = varint::read_varint(input)?;
        ctx.lookup(idx_u64 as u32, delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::CollectCtx;

    #[test]
    fn collect_then_encode_decode_round_trip() {
        let dots = [
            Dot::new(7, 10),
            Dot::new(99, 3),
            Dot::new(7, 12),
            Dot::new(99, 3),
        ];
        let mut cc = CollectCtx::new();
        for d in &dots {
            d.collect(&mut cc);
        }
        let (table, baselines) = cc.finalize();
        let ec = EncCtx::from_table(&table, baselines.clone());
        let dc = DecCtx {
            actor_table: table,
            baselines,
        };

        let mut buf = Vec::new();
        for d in &dots {
            d.encode(&ec, &mut buf).unwrap();
        }

        let mut slice = &buf[..];
        for &d in &dots {
            let got = Dot::decode(&dc, &mut slice).unwrap();
            assert_eq!(got, d);
        }
        assert!(slice.is_empty());
    }

    #[test]
    fn one_byte_idx_one_byte_delta_for_small() {
        let mut cc = CollectCtx::new();
        let d = Dot::new(7, 10);
        d.collect(&mut cc);
        let (table, baselines) = cc.finalize();
        let ec = EncCtx::from_table(&table, baselines);
        let mut buf = Vec::new();
        d.encode(&ec, &mut buf).unwrap();
        assert_eq!(buf.len(), 2);
    }
}
