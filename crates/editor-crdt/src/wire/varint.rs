use crate::wire::WireError;

pub fn write_varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push(((value & 0x7F) as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

/// `input` is advanced past the consumed bytes; bounded to the 10-byte LEB128 limit
/// (`ceil(64/7)`) to reject malformed continuation chains.
pub fn read_varint(input: &mut &[u8]) -> Result<u64, WireError> {
    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    let start_len = input.len();

    for _ in 0..10 {
        let byte = match input.first() {
            Some(&b) => b,
            None => {
                return Err(WireError::Varint {
                    offset: start_len - input.len(),
                });
            }
        };
        *input = &input[1..];

        value |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        shift += 7;
    }

    Err(WireError::Varint {
        offset: start_len - input.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_boundary_values() {
        for &v in &[
            0u64,
            1,
            127,
            128,
            16_383,
            16_384,
            2_097_151,
            2_097_152,
            u32::MAX as u64,
            u64::MAX,
        ] {
            let mut buf = Vec::new();
            write_varint(v, &mut buf);
            let mut slice = &buf[..];
            let decoded = read_varint(&mut slice).expect("decode");
            assert_eq!(decoded, v, "round-trip failed for {v}");
            assert!(
                slice.is_empty(),
                "trailing bytes for {v}: {} left",
                slice.len()
            );
        }
    }

    #[test]
    fn one_byte_for_small_values() {
        let mut buf = Vec::new();
        write_varint(127, &mut buf);
        assert_eq!(buf.len(), 1);
        write_varint(0, &mut buf);
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn malformed_continuation_chain_errors() {
        let bad = [0x80u8; 11];
        let mut slice = &bad[..];
        let err = read_varint(&mut slice).unwrap_err();
        assert!(matches!(err, WireError::Varint { .. }));
    }

    #[test]
    fn truncated_continuation_errors() {
        let bad = [0x80u8];
        let mut slice = &bad[..];
        let err = read_varint(&mut slice).unwrap_err();
        assert!(matches!(err, WireError::Varint { .. }));
    }
}
