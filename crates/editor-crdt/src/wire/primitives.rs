use crate::wire::{DecCtx, EncCtx, Wire, WireError, WireResult, varint};

impl Wire for u8 {
    fn encode(&self, _ctx: &EncCtx, out: &mut Vec<u8>) -> WireResult<()> {
        out.push(*self);
        Ok(())
    }
    fn decode(_ctx: &DecCtx, input: &mut &[u8]) -> WireResult<Self> {
        let &b = input.first().ok_or(WireError::Truncated {
            expected: 1,
            actual: 0,
        })?;
        *input = &input[1..];
        Ok(b)
    }
}

impl Wire for u64 {
    fn encode(&self, _ctx: &EncCtx, out: &mut Vec<u8>) -> WireResult<()> {
        varint::write_varint(*self, out);
        Ok(())
    }
    fn decode(_ctx: &DecCtx, input: &mut &[u8]) -> WireResult<Self> {
        varint::read_varint(input)
    }
}

macro_rules! impl_wire_for_smaller_uint {
    ($($t:ty => $name:literal),*) => {
        $(
            impl Wire for $t {
                fn encode(&self, _ctx: &EncCtx, out: &mut Vec<u8>) -> WireResult<()> {
                    varint::write_varint(*self as u64, out);
                    Ok(())
                }
                fn decode(_ctx: &DecCtx, input: &mut &[u8]) -> WireResult<Self> {
                    let v = varint::read_varint(input)?;
                    if v > <$t>::MAX as u64 {
                        return Err(WireError::IntOverflow {
                            ty: $name,
                            value: v,
                            max: <$t>::MAX as u64,
                        });
                    }
                    Ok(v as $t)
                }
            }
        )*
    };
}

impl_wire_for_smaller_uint!(u16 => "u16", u32 => "u32");

macro_rules! impl_wire_for_signed {
    ($($t:ty => $u:ty, $name:literal),*) => {
        $(
            impl Wire for $t {
                fn encode(&self, _ctx: &EncCtx, out: &mut Vec<u8>) -> WireResult<()> {
                    let bits = <$t>::BITS;
                    let z = ((*self as $u) << 1) ^ ((*self >> (bits - 1)) as $u);
                    varint::write_varint(z as u64, out);
                    Ok(())
                }
                fn decode(_ctx: &DecCtx, input: &mut &[u8]) -> WireResult<Self> {
                    let v = varint::read_varint(input)?;
                    if v > <$u>::MAX as u64 {
                        return Err(WireError::IntOverflow {
                            ty: $name,
                            value: v,
                            max: <$u>::MAX as u64,
                        });
                    }
                    let v = v as $u;
                    let n = ((v >> 1) as $t) ^ -((v & 1) as $t);
                    Ok(n)
                }
            }
        )*
    };
}

impl_wire_for_signed!(i16 => u16, "i16", i32 => u32, "i32", i64 => u64, "i64");

impl Wire for bool {
    fn encode(&self, _ctx: &EncCtx, out: &mut Vec<u8>) -> WireResult<()> {
        out.push(if *self { 1 } else { 0 });
        Ok(())
    }
    fn decode(ctx: &DecCtx, input: &mut &[u8]) -> WireResult<Self> {
        let b = u8::decode(ctx, input)?;
        match b {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(WireError::InvalidBool { tag: n }),
        }
    }
}

impl Wire for String {
    fn encode(&self, ctx: &EncCtx, out: &mut Vec<u8>) -> WireResult<()> {
        let bytes = self.as_bytes();
        (bytes.len() as u64).encode(ctx, out)?;
        out.extend_from_slice(bytes);
        Ok(())
    }
    fn decode(ctx: &DecCtx, input: &mut &[u8]) -> WireResult<Self> {
        let len = u64::decode(ctx, input)? as usize;
        if input.len() < len {
            return Err(WireError::Truncated {
                expected: len,
                actual: input.len(),
            });
        }
        let (bytes, rest) = input.split_at(len);
        let s = std::str::from_utf8(bytes)
            .map_err(|e| WireError::RunUtf8(e.to_string()))?
            .to_owned();
        *input = rest;
        Ok(s)
    }
}

impl<T: Wire> Wire for Option<T> {
    fn collect(&self, ctx: &mut crate::wire::CollectCtx) {
        if let Some(v) = self {
            v.collect(ctx);
        }
    }
    fn encode(&self, ctx: &EncCtx, out: &mut Vec<u8>) -> WireResult<()> {
        match self {
            None => {
                out.push(0);
                Ok(())
            }
            Some(v) => {
                out.push(1);
                v.encode(ctx, out)
            }
        }
    }
    fn decode(ctx: &DecCtx, input: &mut &[u8]) -> WireResult<Self> {
        let tag = u8::decode(ctx, input)?;
        match tag {
            0 => Ok(None),
            1 => Ok(Some(T::decode(ctx, input)?)),
            n => Err(WireError::UnknownVariant {
                ty: "Option",
                tag: n,
            }),
        }
    }
}

impl<T: Wire> Wire for Vec<T> {
    fn collect(&self, ctx: &mut crate::wire::CollectCtx) {
        for v in self {
            v.collect(ctx);
        }
    }
    fn encode(&self, ctx: &EncCtx, out: &mut Vec<u8>) -> WireResult<()> {
        (self.len() as u64).encode(ctx, out)?;
        for v in self {
            v.encode(ctx, out)?;
        }
        Ok(())
    }
    fn decode(ctx: &DecCtx, input: &mut &[u8]) -> WireResult<Self> {
        let len = u64::decode(ctx, input)? as usize;
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            out.push(T::decode(ctx, input)?);
        }
        Ok(out)
    }
}

impl Wire for char {
    fn encode(&self, _ctx: &EncCtx, out: &mut Vec<u8>) -> WireResult<()> {
        let mut buf = [0u8; 4];
        let s = self.encode_utf8(&mut buf);
        out.extend_from_slice(s.as_bytes());
        Ok(())
    }
    fn decode(_ctx: &DecCtx, input: &mut &[u8]) -> WireResult<Self> {
        let b0 = *input.first().ok_or(WireError::Truncated {
            expected: 1,
            actual: 0,
        })?;
        let len = if b0 < 0x80 {
            1
        } else if b0 < 0xC2 {
            return Err(WireError::RunUtf8("invalid UTF-8 leading byte".to_owned()));
        } else if b0 < 0xE0 {
            2
        } else if b0 < 0xF0 {
            3
        } else if b0 <= 0xF4 {
            4
        } else {
            return Err(WireError::RunUtf8("invalid UTF-8 leading byte".to_owned()));
        };
        if input.len() < len {
            return Err(WireError::Truncated {
                expected: len,
                actual: input.len(),
            });
        }
        let (head, tail) = input.split_at(len);
        let s = std::str::from_utf8(head).map_err(|e| WireError::RunUtf8(e.to_string()))?;
        let c = s
            .chars()
            .next()
            .expect("UTF-8 boundary parse guarantees one char");
        *input = tail;
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_ctx() -> (EncCtx, DecCtx) {
        (
            EncCtx::from_table(&[], vec![]),
            DecCtx {
                actor_table: vec![],
                baselines: vec![],
            },
        )
    }

    fn round<T: Wire + std::fmt::Debug + PartialEq>(v: T) {
        let (ec, dc) = empty_ctx();
        let mut buf = Vec::new();
        v.encode(&ec, &mut buf).unwrap();
        let mut slice = &buf[..];
        let decoded: T = T::decode(&dc, &mut slice).unwrap();
        assert_eq!(decoded, v);
        assert!(slice.is_empty());
    }

    #[test]
    fn u8_round_trip() {
        round(0u8);
        round(255u8);
    }
    #[test]
    fn u16_round_trip() {
        round(0u16);
        round(u16::MAX);
    }
    #[test]
    fn u32_round_trip() {
        round(0u32);
        round(u32::MAX);
    }
    #[test]
    fn u64_round_trip() {
        round(0u64);
        round(u64::MAX);
    }
    #[test]
    fn i32_round_trip() {
        round(0i32);
        round(-1i32);
        round(i32::MIN);
        round(i32::MAX);
        round(-1234i32);
    }
    #[test]
    fn i32_zigzag_small_negative_one_byte() {
        let (ec, _) = empty_ctx();
        let mut buf = Vec::new();
        (-1i32).encode(&ec, &mut buf).unwrap();
        assert_eq!(buf.len(), 1, "zigzag -1 → varint(1) → 1 byte");
    }
    #[test]
    fn bool_round_trip() {
        round(true);
        round(false);
    }
    #[test]
    fn string_round_trip() {
        round(String::new());
        round("hello".to_owned());
        round("한글 가나다".to_owned());
    }
    #[test]
    fn option_round_trip() {
        round(None::<u32>);
        round(Some(42u32));
    }
    #[test]
    fn vec_round_trip() {
        round(Vec::<u32>::new());
        round(vec![1u32, 2, 3]);
        round(vec!["a".to_owned(), "b".to_owned()]);
    }
    #[test]
    fn u16_overflow_errors() {
        let (_, dc) = empty_ctx();
        let mut buf = Vec::new();
        varint::write_varint(70_000, &mut buf);
        let mut slice = &buf[..];
        let err = u16::decode(&dc, &mut slice).unwrap_err();
        assert!(matches!(err, WireError::IntOverflow { ty: "u16", .. }));
    }
    #[test]
    fn invalid_bool_errors() {
        let (_, dc) = empty_ctx();
        let buf = vec![2u8];
        let mut slice = &buf[..];
        let err = bool::decode(&dc, &mut slice).unwrap_err();
        assert!(matches!(err, WireError::InvalidBool { tag: 2 }));
    }
    #[test]
    fn truncated_string_errors() {
        let (_, dc) = empty_ctx();
        let mut buf = Vec::new();
        varint::write_varint(10, &mut buf);
        buf.extend_from_slice(b"abc");
        let mut slice = &buf[..];
        let err = String::decode(&dc, &mut slice).unwrap_err();
        assert!(matches!(err, WireError::Truncated { .. }));
    }

    #[test]
    fn char_round_trip_ascii_and_unicode() {
        round('a');
        round('가');
        round('🦀');
    }

    #[test]
    fn char_decode_invalid_lead_byte_errors() {
        let (_, dc) = empty_ctx();
        let bad = vec![0xC0u8];
        let mut slice = &bad[..];
        let err = char::decode(&dc, &mut slice).unwrap_err();
        assert!(matches!(err, WireError::RunUtf8(_)));
    }
}
