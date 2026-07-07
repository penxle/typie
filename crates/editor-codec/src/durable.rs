use editor_crdt::Dot;

use crate::ctx::{CollectCtx, DecCtx, EncCtx, read_dot, write_dot};
use crate::error::{CodecResult, Corruption};
use crate::primitives::{
    read_bool, read_char, read_option, read_string, read_u8, read_vec, write_bool, write_char,
    write_option, write_string, write_u8, write_vec,
};
use crate::varint::{decode_zigzag, encode_zigzag, read_varint, write_varint};

pub trait Durable: Sized {
    fn collect(&self, cc: &mut CollectCtx) {
        let _ = cc;
    }
    fn encode(&self, ctx: &EncCtx, out: &mut Vec<u8>) -> CodecResult<()>;
    fn decode(ctx: &DecCtx, input: &mut &[u8]) -> CodecResult<Self>;
}

macro_rules! impl_varint_uint {
    ($($t:ty),*) => {
        $(
            impl Durable for $t {
                fn encode(&self, _ctx: &EncCtx, out: &mut Vec<u8>) -> CodecResult<()> {
                    write_varint(*self as u64, out);
                    Ok(())
                }
                fn decode(_ctx: &DecCtx, input: &mut &[u8]) -> CodecResult<Self> {
                    let v = read_varint(input)?;
                    <$t>::try_from(v).map_err(|_| Corruption::VarintOverflow.into())
                }
            }
        )*
    };
}

macro_rules! impl_zigzag_int {
    ($($t:ty),*) => {
        $(
            impl Durable for $t {
                fn encode(&self, _ctx: &EncCtx, out: &mut Vec<u8>) -> CodecResult<()> {
                    write_varint(encode_zigzag(*self as i64), out);
                    Ok(())
                }
                fn decode(_ctx: &DecCtx, input: &mut &[u8]) -> CodecResult<Self> {
                    let v = decode_zigzag(read_varint(input)?);
                    <$t>::try_from(v).map_err(|_| Corruption::VarintOverflow.into())
                }
            }
        )*
    };
}

impl_varint_uint!(u16, u32, u64, usize);
impl_zigzag_int!(i16, i32, i64);

impl Durable for u8 {
    fn encode(&self, _ctx: &EncCtx, out: &mut Vec<u8>) -> CodecResult<()> {
        write_u8(*self, out);
        Ok(())
    }
    fn decode(_ctx: &DecCtx, input: &mut &[u8]) -> CodecResult<Self> {
        read_u8(input)
    }
}

impl Durable for bool {
    fn encode(&self, _ctx: &EncCtx, out: &mut Vec<u8>) -> CodecResult<()> {
        write_bool(*self, out);
        Ok(())
    }
    fn decode(_ctx: &DecCtx, input: &mut &[u8]) -> CodecResult<Self> {
        read_bool(input)
    }
}

impl Durable for char {
    fn encode(&self, _ctx: &EncCtx, out: &mut Vec<u8>) -> CodecResult<()> {
        write_char(*self, out);
        Ok(())
    }
    fn decode(_ctx: &DecCtx, input: &mut &[u8]) -> CodecResult<Self> {
        read_char(input)
    }
}

impl Durable for String {
    fn encode(&self, _ctx: &EncCtx, out: &mut Vec<u8>) -> CodecResult<()> {
        write_string(self, out);
        Ok(())
    }
    fn decode(_ctx: &DecCtx, input: &mut &[u8]) -> CodecResult<Self> {
        read_string(input)
    }
}

impl<T: Durable> Durable for Option<T> {
    fn collect(&self, cc: &mut CollectCtx) {
        if let Some(v) = self {
            v.collect(cc);
        }
    }
    fn encode(&self, ctx: &EncCtx, out: &mut Vec<u8>) -> CodecResult<()> {
        write_option(self, out, |v, o| v.encode(ctx, o))
    }
    fn decode(ctx: &DecCtx, input: &mut &[u8]) -> CodecResult<Self> {
        read_option(input, |i| T::decode(ctx, i))
    }
}

impl<T: Durable> Durable for Vec<T> {
    fn collect(&self, cc: &mut CollectCtx) {
        for v in self {
            v.collect(cc);
        }
    }
    fn encode(&self, ctx: &EncCtx, out: &mut Vec<u8>) -> CodecResult<()> {
        write_vec(self, out, |v, o| v.encode(ctx, o))
    }
    fn decode(ctx: &DecCtx, input: &mut &[u8]) -> CodecResult<Self> {
        read_vec(input, |i| T::decode(ctx, i))
    }
}

impl Durable for Dot {
    fn collect(&self, cc: &mut CollectCtx) {
        cc.observe(self);
    }
    fn encode(&self, ctx: &EncCtx, out: &mut Vec<u8>) -> CodecResult<()> {
        write_dot(self, ctx, out)
    }
    fn decode(ctx: &DecCtx, input: &mut &[u8]) -> CodecResult<Self> {
        read_dot(input, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodecError;

    pub(crate) fn round_trip<T: Durable + PartialEq + std::fmt::Debug>(value: &T) -> T {
        let mut cc = CollectCtx::new();
        value.collect(&mut cc);
        let (actors, baselines) = cc.finalize();
        let enc = EncCtx::from_parts(&actors, baselines.clone()).unwrap();
        let dec = DecCtx { actors, baselines };
        let mut buf = Vec::new();
        value.encode(&enc, &mut buf).unwrap();
        let mut slice = &buf[..];
        let out = T::decode(&dec, &mut slice).unwrap();
        assert!(slice.is_empty(), "trailing bytes after decode");
        out
    }

    #[test]
    fn uint_round_trips() {
        assert_eq!(round_trip(&0u16), 0);
        assert_eq!(round_trip(&u16::MAX), u16::MAX);
        assert_eq!(round_trip(&u32::MAX), u32::MAX);
        assert_eq!(round_trip(&u64::MAX), u64::MAX);
        assert_eq!(round_trip(&usize::MAX), usize::MAX);
        assert_eq!(round_trip(&7u8), 7);
    }

    #[test]
    fn uint_decode_rejects_over_width() {
        let mut buf = Vec::new();
        write_varint(u64::from(u16::MAX) + 1, &mut buf);
        let dec = DecCtx {
            actors: vec![],
            baselines: vec![],
        };
        let mut slice = &buf[..];
        assert!(matches!(
            <u16 as Durable>::decode(&dec, &mut slice),
            Err(CodecError::Corruption(Corruption::VarintOverflow))
        ));
    }

    #[test]
    fn int_round_trips() {
        assert_eq!(round_trip(&i16::MIN), i16::MIN);
        assert_eq!(round_trip(&-1i32), -1);
        assert_eq!(round_trip(&i64::MIN), i64::MIN);
        assert_eq!(round_trip(&i64::MAX), i64::MAX);
    }

    #[test]
    fn misc_round_trips() {
        assert!(round_trip(&true));
        assert_eq!(round_trip(&'한'), '한');
        assert_eq!(round_trip(&"타이피".to_owned()), "타이피");
        assert_eq!(round_trip(&Some(300u64)), Some(300));
        assert_eq!(round_trip(&None::<String>), None);
        assert_eq!(round_trip(&vec![1u64, 2, 3]), vec![1, 2, 3]);
    }

    #[test]
    fn dot_round_trips_through_ctx() {
        let dot = Dot::new(7, 10);
        assert_eq!(round_trip(&dot), dot);
        let dots = vec![Dot::new(7, 10), Dot::new(99, 3), Dot::new(7, 12)];
        assert_eq!(round_trip(&dots), dots);
    }

    mod derive_smoke {
        use super::*;
        use crate::framing::UnknownTail;
        use editor_codec_macros::Durable;

        #[derive(Debug, PartialEq, Durable)]
        #[durable(evolvable)]
        struct SelfHosted {
            a: u64,
            tail: UnknownTail,
        }

        #[test]
        fn derive_resolves_inside_editor_codec() {
            let v = SelfHosted {
                a: 300,
                tail: UnknownTail::default(),
            };
            assert_eq!(round_trip(&v), v);
        }
    }
}
