use crate::error::{CodecResult, Corruption};

pub fn write_varint(mut v: u64, out: &mut Vec<u8>) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

pub fn read_varint(input: &mut &[u8]) -> CodecResult<u64> {
    let mut result = 0u64;
    let mut shift = 0u32;
    loop {
        let (&byte, rest) = input.split_first().ok_or(Corruption::Truncated {
            expected: 1,
            actual: 0,
        })?;
        *input = rest;
        if shift == 63 && byte > 0x01 {
            return Err(Corruption::VarintOverflow.into());
        }
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            // canonical 강제: 마지막 바이트가 0이면(단일 바이트 0 제외) 비정준 표기 —
            // 동일 값의 유일 표기가 재인코드 바이트 안정성의 전제다
            if shift > 0 && byte == 0 {
                return Err(Corruption::NonCanonicalVarint.into());
            }
            return Ok(result);
        }
        shift += 7;
        if shift > 63 {
            return Err(Corruption::VarintOverflow.into());
        }
    }
}

pub fn encode_zigzag(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

pub fn decode_zigzag(v: u64) -> i64 {
    ((v >> 1) as i64) ^ -((v & 1) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn varint_known_bytes() {
        let cases: &[(u64, &[u8])] = &[
            (0, &[0x00]),
            (1, &[0x01]),
            (127, &[0x7f]),
            (128, &[0x80, 0x01]),
            (300, &[0xac, 0x02]),
            (
                u64::MAX,
                &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01],
            ),
        ];
        for (v, bytes) in cases {
            let mut out = Vec::new();
            write_varint(*v, &mut out);
            assert_eq!(&out[..], *bytes, "encode {v}");
            let mut slice = &out[..];
            assert_eq!(read_varint(&mut slice).unwrap(), *v, "decode {v}");
            assert!(slice.is_empty());
        }
    }

    #[test]
    fn varint_truncated_errors() {
        let mut slice: &[u8] = &[0x80];
        assert!(matches!(
            read_varint(&mut slice),
            Err(crate::CodecError::Corruption(Corruption::Truncated { .. }))
        ));
        let mut empty: &[u8] = &[];
        assert!(matches!(
            read_varint(&mut empty),
            Err(crate::CodecError::Corruption(Corruption::Truncated { .. }))
        ));
    }

    #[test]
    fn varint_overflow_errors() {
        let mut slice: &[u8] = &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02];
        assert!(matches!(
            read_varint(&mut slice),
            Err(crate::CodecError::Corruption(Corruption::VarintOverflow))
        ));
        let mut eleven: &[u8] = &[0xff; 11];
        assert!(matches!(
            read_varint(&mut eleven),
            Err(crate::CodecError::Corruption(Corruption::VarintOverflow))
        ));
    }

    #[test]
    fn noncanonical_varint_is_rejected() {
        // 비정준 표기(불필요한 continuation)는 재인코드 바이트 안정성의 전제를 깨므로 거부
        for bytes in [
            &[0x80u8, 0x00][..],
            &[0x81, 0x00][..],
            &[0xff, 0x80, 0x00][..],
        ] {
            let mut slice = bytes;
            assert!(
                matches!(
                    read_varint(&mut slice),
                    Err(crate::CodecError::Corruption(
                        Corruption::NonCanonicalVarint
                    ))
                ),
                "{bytes:?}"
            );
        }
    }

    #[test]
    fn zigzag_known_values() {
        assert_eq!(encode_zigzag(0), 0);
        assert_eq!(encode_zigzag(-1), 1);
        assert_eq!(encode_zigzag(1), 2);
        assert_eq!(encode_zigzag(i64::MIN), u64::MAX);
        assert_eq!(decode_zigzag(u64::MAX), i64::MIN);
    }

    proptest! {
        #[test]
        fn varint_round_trip(v in any::<u64>()) {
            let mut out = Vec::new();
            write_varint(v, &mut out);
            let mut slice = &out[..];
            prop_assert_eq!(read_varint(&mut slice).unwrap(), v);
            prop_assert!(slice.is_empty());
        }

        #[test]
        fn zigzag_round_trip(v in any::<i64>()) {
            prop_assert_eq!(decode_zigzag(encode_zigzag(v)), v);
        }
    }
}
