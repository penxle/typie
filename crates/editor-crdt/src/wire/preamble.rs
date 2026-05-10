use crate::wire::{DecCtx, WireError, WireResult, varint};

pub fn encode_preamble(actor_table: &[u64], baselines: &[u64], out: &mut Vec<u8>) {
    debug_assert_eq!(actor_table.len(), baselines.len());
    varint::write_varint(actor_table.len() as u64, out);
    for &a in actor_table {
        out.extend_from_slice(&a.to_le_bytes());
    }
    for &b in baselines {
        varint::write_varint(b, out);
    }
}

pub fn decode_preamble(input: &mut &[u8]) -> WireResult<DecCtx> {
    let count = varint::read_varint(input)? as usize;
    if input.len() < count * 8 {
        return Err(WireError::Truncated {
            expected: count * 8,
            actual: input.len(),
        });
    }
    let mut actor_table = Vec::with_capacity(count);
    for _ in 0..count {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&input[..8]);
        actor_table.push(u64::from_le_bytes(buf));
        *input = &input[8..];
    }
    let mut baselines = Vec::with_capacity(count);
    for _ in 0..count {
        baselines.push(varint::read_varint(input)?);
    }
    Ok(DecCtx {
        actor_table,
        baselines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_two_actors() {
        let actors = vec![7u64, 99u64];
        let baselines = vec![3u64, 5u64];
        let mut buf = Vec::new();
        encode_preamble(&actors, &baselines, &mut buf);
        let mut slice = &buf[..];
        let dc = decode_preamble(&mut slice).unwrap();
        assert_eq!(dc.actor_table, actors);
        assert_eq!(dc.baselines, baselines);
        assert!(slice.is_empty());
    }

    #[test]
    fn round_trip_empty() {
        let mut buf = Vec::new();
        encode_preamble(&[], &[], &mut buf);
        let mut slice = &buf[..];
        let dc = decode_preamble(&mut slice).unwrap();
        assert!(dc.actor_table.is_empty());
        assert!(dc.baselines.is_empty());
    }
}
