use crate::error::{CodecResult, Corruption};
use crate::primitives::read_len_prefixed;
use crate::varint::{read_varint, write_varint};

/// 계약: 내부에 preamble-상대 Dot 인코딩이 있을 수 있으므로 **원본 preamble 컨텍스트
/// 안에서만** 재방출 가능 — 다른 번들로의 이식 금지. 구조적 봉인은 번들 계층(Plan 4)이
/// 소유 스코프 타입으로 강제한다.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnknownTail(pub Vec<u8>);

/// 계약: `UnknownTail`과 동일 — 원본 preamble 컨텍스트 밖으로 이식 금지.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownPayload {
    pub tag: u64,
    pub bytes: Vec<u8>,
}

pub fn write_frame(
    out: &mut Vec<u8>,
    f: impl FnOnce(&mut Vec<u8>) -> CodecResult<()>,
) -> CodecResult<()> {
    let mut body = Vec::new();
    f(&mut body)?;
    write_varint(body.len() as u64, out);
    out.extend_from_slice(&body);
    Ok(())
}

pub struct FrameReader<'a> {
    rest: &'a [u8],
}

impl<'a> FrameReader<'a> {
    pub fn open(input: &mut &'a [u8]) -> CodecResult<Self> {
        Ok(Self {
            rest: read_len_prefixed(input)?,
        })
    }

    pub fn from_body(body: &'a [u8]) -> Self {
        Self { rest: body }
    }

    pub fn try_field<T>(
        &mut self,
        f: impl FnOnce(&mut &'a [u8]) -> CodecResult<T>,
    ) -> CodecResult<Option<T>> {
        if self.rest.is_empty() {
            return Ok(None);
        }
        let before = self.rest.len();
        let value = f(&mut self.rest)?;
        if self.rest.len() == before {
            // 0바이트 필드는 "존재 여부"가 뒤따르는 꼬리 유무에 좌우되어 위치 파싱을
            // 오염시킨다 (Plan 2 매크로는 0바이트 인코딩 필드 타입 자체를 금지)
            return Err(Corruption::NoProgress.into());
        }
        Ok(Some(value))
    }

    pub fn capture_tail(self) -> Vec<u8> {
        self.rest.to_vec()
    }
}

pub fn expect_consumed(rest: &[u8]) -> CodecResult<()> {
    if rest.is_empty() {
        Ok(())
    } else {
        Err(Corruption::TrailingBytes {
            remaining: rest.len(),
        }
        .into())
    }
}

pub fn write_tail(tail: &UnknownTail, out: &mut Vec<u8>) {
    out.extend_from_slice(&tail.0);
}

pub fn write_open_variant(
    tag: u64,
    out: &mut Vec<u8>,
    f: impl FnOnce(&mut Vec<u8>) -> CodecResult<()>,
) -> CodecResult<()> {
    write_varint(tag, out);
    write_frame(out, f)
}

pub fn read_open_variant<'a>(input: &mut &'a [u8]) -> CodecResult<(u64, &'a [u8])> {
    let tag = read_varint(input)?;
    let body = read_len_prefixed(input)?;
    Ok((tag, body))
}

pub fn write_unknown_variant(u: &UnknownPayload, out: &mut Vec<u8>) -> CodecResult<()> {
    write_varint(u.tag, out);
    write_varint(u.bytes.len() as u64, out);
    out.extend_from_slice(&u.bytes);
    Ok(())
}

pub fn write_closed_tag(tag: u64, out: &mut Vec<u8>) {
    write_varint(tag, out);
}

pub fn read_closed_tag(input: &mut &[u8]) -> CodecResult<u64> {
    read_varint(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{read_option, read_string, write_option, write_string};
    use crate::varint::{read_varint, write_varint};
    use proptest::prelude::*;

    // 시뮬레이션: v1 = { a: u64 }, v2 = { a: u64, b: Option<String> } (b는 v2에서 append된 필드)
    #[derive(Debug, PartialEq)]
    struct V1 {
        a: u64,
        tail: UnknownTail,
    }

    #[derive(Debug, PartialEq)]
    struct V2 {
        a: u64,
        b: Option<String>,
    }

    fn encode_v1(v: &V1, out: &mut Vec<u8>) -> crate::CodecResult<()> {
        write_frame(out, |body| {
            write_varint(v.a, body);
            write_tail(&v.tail, body);
            Ok(())
        })
    }

    fn decode_v1(input: &mut &[u8]) -> crate::CodecResult<V1> {
        let mut frame = FrameReader::open(input)?;
        let a = frame.try_field(read_varint)?.unwrap_or(0);
        let tail = UnknownTail(frame.capture_tail());
        Ok(V1 { a, tail })
    }

    fn encode_v2(v: &V2, out: &mut Vec<u8>) -> crate::CodecResult<()> {
        write_frame(out, |body| {
            write_varint(v.a, body);
            // 정준 규칙: 자기 스키마의 모든 필드를 항상 인코딩 — 기본값 생략 금지
            write_option(&v.b, body, |s, o| {
                write_string(s, o);
                Ok(())
            })
        })
    }

    fn decode_v2(input: &mut &[u8]) -> crate::CodecResult<V2> {
        let mut frame = FrameReader::open(input)?;
        let a = frame.try_field(read_varint)?.unwrap_or(0);
        let b = frame
            .try_field(|input| read_option(input, read_string))?
            .flatten();
        let _ = frame.capture_tail();
        Ok(V2 { a, b })
    }

    #[test]
    fn old_reader_preserves_new_tail_byte_stable() {
        // v2 라이터 → v1 리더 → 재인코드가 바이트 동일 (스펙의 재인코드 바이트 안정성)
        let v2 = V2 {
            a: 42,
            b: Some("new-field".to_owned()),
        };
        let mut v2_bytes = Vec::new();
        encode_v2(&v2, &mut v2_bytes).unwrap();

        let mut slice = &v2_bytes[..];
        let v1 = decode_v1(&mut slice).unwrap();
        assert!(slice.is_empty());
        assert_eq!(v1.a, 42);
        assert!(!v1.tail.0.is_empty());

        let mut reencoded = Vec::new();
        encode_v1(&v1, &mut reencoded).unwrap();
        assert_eq!(reencoded, v2_bytes);
    }

    #[test]
    fn new_reader_defaults_missing_tail_fields() {
        // v1 라이터 → v2 리더: 누락 필드 b → None
        let v1 = V1 {
            a: 7,
            tail: UnknownTail::default(),
        };
        let mut bytes = Vec::new();
        encode_v1(&v1, &mut bytes).unwrap();
        let mut slice = &bytes[..];
        let v2 = decode_v2(&mut slice).unwrap();
        assert_eq!(v2, V2 { a: 7, b: None });
    }

    #[test]
    fn open_variant_known_round_trip() {
        let mut out = Vec::new();
        write_open_variant(3, &mut out, |body| {
            write_string("payload", body);
            Ok(())
        })
        .unwrap();
        let mut slice = &out[..];
        let (tag, mut body) = read_open_variant(&mut slice).unwrap();
        assert_eq!(tag, 3);
        assert_eq!(read_string(&mut body).unwrap(), "payload");
        expect_consumed(body).unwrap();
        assert!(slice.is_empty());
    }

    #[test]
    fn known_variant_frozen_payload_rejects_trailing_bytes() {
        let mut out = Vec::new();
        write_open_variant(1, &mut out, |body| {
            write_string("payload", body);
            body.push(0xff); // 프로토콜 위반: frozen payload 뒤 잉여 바이트
            Ok(())
        })
        .unwrap();
        let mut slice = &out[..];
        let (_tag, mut body) = read_open_variant(&mut slice).unwrap();
        read_string(&mut body).unwrap();
        assert!(matches!(
            expect_consumed(body),
            Err(crate::CodecError::Corruption(Corruption::TrailingBytes {
                remaining: 1
            }))
        ));
    }

    #[test]
    fn known_variant_evolvable_payload_preserves_appended_fields_byte_stable() {
        // 신버전이 variant payload(evolvable 필드 목록)에 필드를 append한 경우:
        // 구버전은 아는 필드만 읽고 꼬리를 보존하며, 재인코드는 바이트 동일해야 한다.
        let mut original = Vec::new();
        write_open_variant(2, &mut original, |body| {
            write_varint(42, body);
            write_string("appended-by-vnext", body); // 구버전이 모르는 append 필드
            Ok(())
        })
        .unwrap();

        let mut slice = &original[..];
        let (tag, body) = read_open_variant(&mut slice).unwrap();
        let mut frame = FrameReader::from_body(body);
        let a = frame.try_field(read_varint).unwrap().unwrap();
        let tail = UnknownTail(frame.capture_tail());
        assert_eq!(a, 42);
        assert!(!tail.0.is_empty());

        let mut reencoded = Vec::new();
        write_open_variant(tag, &mut reencoded, |body| {
            write_varint(a, body);
            write_tail(&tail, body);
            Ok(())
        })
        .unwrap();
        assert_eq!(reencoded, original);
    }

    #[test]
    fn open_variant_unknown_preserved_byte_stable() {
        let mut original = Vec::new();
        write_open_variant(9999, &mut original, |body| {
            body.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
            Ok(())
        })
        .unwrap();

        let mut slice = &original[..];
        let (tag, body) = read_open_variant(&mut slice).unwrap();
        let unknown = UnknownPayload {
            tag,
            bytes: body.to_vec(),
        };

        let mut reencoded = Vec::new();
        write_unknown_variant(&unknown, &mut reencoded).unwrap();
        assert_eq!(reencoded, original);
    }

    #[test]
    fn closed_tag_round_trip() {
        let mut out = Vec::new();
        write_closed_tag(1, &mut out);
        let mut slice = &out[..];
        assert_eq!(read_closed_tag(&mut slice).unwrap(), 1);
    }

    #[test]
    fn zero_progress_field_decoder_is_rejected() {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, |body| {
            body.push(0xaa);
            Ok(())
        })
        .unwrap();
        let mut slice = &bytes[..];
        let mut frame = FrameReader::open(&mut slice).unwrap();
        assert!(matches!(
            frame.try_field(|_input| Ok(0u8)),
            Err(crate::CodecError::Corruption(Corruption::NoProgress))
        ));
    }

    proptest! {
        #[test]
        fn unknown_tail_always_byte_stable(
            a in any::<u64>(),
            tail in proptest::collection::vec(any::<u8>(), 0..256),
        ) {
            // 임의 꼬리 바이트가 v1 왕복을 그대로 관통한다
            let mut bytes = Vec::new();
            write_frame(&mut bytes, |body| {
                write_varint(a, body);
                body.extend_from_slice(&tail);
                Ok(())
            }).unwrap();

            let mut slice = &bytes[..];
            let v1 = decode_v1(&mut slice).unwrap();
            let mut reencoded = Vec::new();
            encode_v1(&v1, &mut reencoded).unwrap();
            prop_assert_eq!(reencoded, bytes);
        }

        #[test]
        fn framing_arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
            let mut slice = &bytes[..];
            let _ = read_open_variant(&mut slice);
            let mut slice = &bytes[..];
            if let Ok(mut frame) = FrameReader::open(&mut slice) {
                let _ = frame.try_field(read_varint);
                let _ = frame.capture_tail();
            }
        }
    }
}
