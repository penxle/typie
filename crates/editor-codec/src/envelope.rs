use std::io::Read;

use crate::error::{CodecResult, Corruption, EncodeInvariant, Fenced};
use crate::primitives::{read_u8, take};
use crate::varint::{read_varint, write_varint};

pub const MAGIC: u8 = 0xC2;
pub const FORMAT_VERSION: u8 = 1;
pub const SUPPORTED_REQUIRED_FEATURES: u64 = 0;
pub const COMPRESSION_THRESHOLD_BYTES: usize = 256;
pub const FLAG_COMPRESSED: u8 = 0b0000_0001;
pub const KNOWN_FLAGS: u8 = FLAG_COMPRESSED;
pub const MAX_BODY_BYTES: u64 = 1 << 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PayloadKind {
    ChangesetBundle = 0,
    Dots = 1,
    Snapshot = 2,
}

impl PayloadKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(PayloadKind::ChangesetBundle),
            1 => Some(PayloadKind::Dots),
            2 => Some(PayloadKind::Snapshot),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub payload_kind: PayloadKind,
    pub epoch: u64,
    pub required_features: u64,
    pub optional_features: u64,
    pub body: Vec<u8>,
}

impl Envelope {
    pub fn new(payload_kind: PayloadKind, body: Vec<u8>) -> Self {
        Self {
            payload_kind,
            epoch: 0,
            required_features: 0,
            optional_features: 0,
            body,
        }
    }
}

pub fn wrap(envelope: &Envelope) -> CodecResult<Vec<u8>> {
    // 라이터 불변식: 현재 라이터가 표현할 수 없는 값은 쓰기 전에 거부한다 —
    // 자기 리더가 못 여는 envelope를 생산하는 경로를 타입 수준에서 봉쇄
    let unsupported = envelope.required_features & !SUPPORTED_REQUIRED_FEATURES;
    if unsupported != 0 {
        return Err(EncodeInvariant::UnsupportedRequiredFeatures { bits: unsupported }.into());
    }
    if envelope.epoch != 0 {
        return Err(EncodeInvariant::UnsupportedEpoch {
            got: envelope.epoch,
        }
        .into());
    }
    writer_body_cap(envelope.body.len() as u64)?;

    let (flags, stored_body, raw_len): (u8, Vec<u8>, Option<u64>) =
        if envelope.body.len() >= COMPRESSION_THRESHOLD_BYTES {
            let compressed = ruzstd::encoding::compress_to_vec(
                envelope.body.as_slice(),
                ruzstd::encoding::CompressionLevel::Fastest,
            );
            if compressed.len() < envelope.body.len() {
                (
                    FLAG_COMPRESSED,
                    compressed,
                    Some(envelope.body.len() as u64),
                )
            } else {
                (0, envelope.body.clone(), None)
            }
        } else {
            (0, envelope.body.clone(), None)
        };

    let mut out = Vec::with_capacity(stored_body.len() + 32);
    out.push(MAGIC);
    out.push(FORMAT_VERSION);
    write_varint(envelope.required_features, &mut out);
    write_varint(envelope.optional_features, &mut out);
    write_varint(envelope.epoch, &mut out);
    out.push(envelope.payload_kind as u8);
    out.push(flags);
    write_varint(stored_body.len() as u64, &mut out);
    if let Some(raw) = raw_len {
        write_varint(raw, &mut out);
    }

    // 체크섬 = checksum 필드 자신을 제외한 전체 (헤더 프리픽스 + 저장 body)
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    hasher.update(&out);
    hasher.update(&stored_body);
    out.extend_from_slice(&hasher.digest().to_le_bytes());
    out.extend_from_slice(&stored_body);
    Ok(out)
}

pub fn unwrap_one(input: &mut &[u8]) -> CodecResult<Envelope> {
    let original = *input;
    let magic = read_u8(input)?;
    if magic != MAGIC {
        return Err(Corruption::BadMagic { got: magic }.into());
    }
    let version = read_u8(input)?;
    if version != FORMAT_VERSION {
        return Err(Fenced::FormatVersion {
            got: version,
            supported: FORMAT_VERSION,
        }
        .into());
    }
    let required_features = read_varint(input)?;
    // 펜싱은 파스 형태를 결정하는 필드 직후, 나머지 헤더 해석 전에 수행한다 —
    // v-next가 새 flag·새 조건부 헤더 필드를 도입해도(새 flag은 required bit 동반 의무)
    // 구 리더는 형태 의존적 파스에 도달하기 전에 여기서 Fenced로 멈춘다.
    let unknown_bits = required_features & !SUPPORTED_REQUIRED_FEATURES;
    if unknown_bits != 0 {
        return Err(Fenced::RequiredFeatures { unknown_bits }.into());
    }
    let optional_features = read_varint(input)?;
    let epoch = read_varint(input)?;
    let kind_byte = read_u8(input)?;
    let flags = read_u8(input)?;
    let body_len = read_varint(input)?;
    if body_len > MAX_BODY_BYTES {
        // 압축/비압축 정책 일관성 — 상한은 단일 상수로 통제
        return Err(Corruption::BodyTooLarge {
            declared: body_len,
            max: MAX_BODY_BYTES,
        }
        .into());
    }
    let raw_len = if flags & FLAG_COMPRESSED != 0 {
        let raw = read_varint(input)?;
        if raw > MAX_BODY_BYTES {
            return Err(Corruption::BodyTooLarge {
                declared: raw,
                max: MAX_BODY_BYTES,
            }
            .into());
        }
        Some(raw)
    } else {
        None
    };
    if body_len > (input.len().saturating_sub(8)) as u64 {
        return Err(Corruption::LengthOverflow {
            declared: body_len,
            remaining: input.len().saturating_sub(8),
        }
        .into());
    }
    let header_len = original.len() - input.len();
    let checksum = u64::from_le_bytes(take(input, 8)?.try_into().expect("8 bytes"));
    let stored_body = take(input, body_len as usize)?;

    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    hasher.update(&original[..header_len]);
    hasher.update(stored_body);
    if hasher.digest() != checksum {
        return Err(Corruption::ChecksumMismatch.into());
    }

    if flags & !KNOWN_FLAGS != 0 {
        // required 펜스를 통과한 뒤에만 도달 — 여기 걸리면 "required bit 없는 새 flag"
        // 즉 규격 위반 라이터 또는 rot이므로 Corruption이 맞다
        return Err(Corruption::ReservedFlagBits {
            got: flags & !KNOWN_FLAGS,
        }
        .into());
    }
    if epoch != 0 {
        return Err(Fenced::Epoch { got: epoch }.into());
    }
    let payload_kind =
        PayloadKind::from_u8(kind_byte).ok_or(Fenced::PayloadKind { got: kind_byte })?;

    let body = if let Some(raw) = raw_len {
        let mut cursor = stored_body;
        // ruzstd 0.8.3의 window 상한(100 MiB) 가드는 state가 이미 있는 reset 경로에만
        // 있고 최초 init 경로에는 없다. seed 프레임으로 state를 선점해 실제 입력이
        // 반드시 가드된 reset 경로를 타게 한다 — 적대적 window 선언(최대 2^41)의
        // 무상한 할당 차단.
        let seed =
            ruzstd::encoding::compress_to_vec(&[][..], ruzstd::encoding::CompressionLevel::Fastest);
        let mut frame_decoder = ruzstd::decoding::FrameDecoder::new();
        frame_decoder
            .init(seed.as_slice())
            .map_err(|e| Corruption::Zstd(format!("{e:?}")))?;
        let decoder =
            ruzstd::decoding::StreamingDecoder::new_with_decoder(&mut cursor, frame_decoder)
                .map_err(|e| Corruption::Zstd(format!("{e:?}")))?;
        let mut body = Vec::new();
        // raw_len + 1로 유계 읽기: 출력 할당·CPU가 선언값에 묶인다
        let mut limited = decoder.take(raw + 1);
        limited
            .read_to_end(&mut body)
            .map_err(|e| Corruption::Zstd(format!("{e:?}")))?;
        if body.len() as u64 != raw {
            return Err(Corruption::RawLenMismatch {
                declared: raw,
                actual: body.len() as u64,
            }
            .into());
        }
        // 정확 소비: 프레임 뒤 잉여 바이트/추가 프레임은 체크섬이 유효해도 거부 —
        // 동일 payload의 복수 표기(밀수 채널)를 봉쇄
        drop(limited);
        if !cursor.is_empty() {
            return Err(Corruption::TrailingBytes {
                remaining: cursor.len(),
            }
            .into());
        }
        body
    } else {
        stored_body.to_vec()
    };

    Ok(Envelope {
        payload_kind,
        epoch,
        required_features,
        optional_features,
        body,
    })
}

pub fn unwrap(bytes: &[u8]) -> CodecResult<Envelope> {
    let mut input = bytes;
    let envelope = unwrap_one(&mut input)?;
    if !input.is_empty() {
        return Err(Corruption::TrailingBytes {
            remaining: input.len(),
        }
        .into());
    }
    Ok(envelope)
}

// wrap의 크기 불변식 — 리더가 거부할 크기를 쓰기 전에 거부.
// stored/raw 둘 다 body.len()에 유계이므로 이 검사 하나로 충분.
// (테스트가 실제 거대 버퍼를 할당하지 않도록 검증을 값 수준으로 분리)
fn writer_body_cap(len: u64) -> CodecResult<()> {
    if len > MAX_BODY_BYTES {
        return Err(EncodeInvariant::BodyTooLarge {
            len,
            max: MAX_BODY_BYTES,
        }
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodecError;
    use proptest::prelude::*;

    fn sample(body: Vec<u8>) -> Envelope {
        Envelope::new(PayloadKind::ChangesetBundle, body)
    }

    #[test]
    fn round_trip_small_uncompressed() {
        let env = sample(b"hello".to_vec());
        let bytes = wrap(&env).unwrap();
        assert_eq!(bytes[0], MAGIC);
        assert_eq!(bytes[1], FORMAT_VERSION);
        assert_eq!(unwrap(&bytes).unwrap(), env);
    }

    #[test]
    fn round_trip_large_compressed() {
        let env = sample(vec![b'x'; 10_000]);
        let bytes = wrap(&env).unwrap();
        assert!(bytes.len() < 5_000, "compressed: {}", bytes.len());
        assert_eq!(unwrap(&bytes).unwrap(), env);
    }

    #[test]
    fn incompressible_body_falls_back_to_uncompressed() {
        let mut body = Vec::with_capacity(512);
        let mut x = 0u64;
        while body.len() < 512 {
            x = xxhash_rust::xxh3::xxh3_64(&x.to_le_bytes());
            body.extend_from_slice(&x.to_le_bytes());
        }
        let env = sample(body);
        let bytes = wrap(&env).unwrap();
        assert_eq!(bytes[6] & FLAG_COMPRESSED, 0);
        assert_eq!(unwrap(&bytes).unwrap(), env);
    }

    #[test]
    fn stream_of_two_envelopes_splits() {
        let a = sample(b"first".to_vec());
        let b = Envelope::new(PayloadKind::Dots, b"second".to_vec());
        let mut stream = wrap(&a).unwrap();
        stream.extend_from_slice(&wrap(&b).unwrap());
        let mut slice = &stream[..];
        assert_eq!(unwrap_one(&mut slice).unwrap(), a);
        assert_eq!(unwrap_one(&mut slice).unwrap(), b);
        assert!(slice.is_empty());
    }

    #[test]
    fn bad_magic_is_corruption() {
        let mut bytes = wrap(&sample(b"x".to_vec())).unwrap();
        bytes[0] = 0xCD; // 구 포맷 magic
        assert!(matches!(
            unwrap(&bytes),
            Err(CodecError::Corruption(Corruption::BadMagic { got: 0xCD }))
        ));
    }

    #[test]
    fn newer_format_version_is_fenced() {
        let mut bytes = wrap(&sample(b"x".to_vec())).unwrap();
        bytes[1] = FORMAT_VERSION + 1;
        assert!(matches!(
            unwrap(&bytes),
            Err(CodecError::Fenced(Fenced::FormatVersion { .. }))
        ));
    }

    #[test]
    fn unknown_required_feature_is_fenced() {
        let bytes = forge(0b100, 0, PayloadKind::ChangesetBundle as u8, 0, b"x", None);
        assert!(matches!(
            unwrap(&bytes),
            Err(CodecError::Fenced(Fenced::RequiredFeatures {
                unknown_bits: 0b100
            }))
        ));
    }

    #[test]
    fn wrap_rejects_writer_inexpressible_envelopes() {
        // 라이터 불변식: 자기 리더가 못 여는 데이터를 쓰는 경로를 봉쇄
        let mut env = sample(b"x".to_vec());
        env.required_features = 0b100;
        assert!(matches!(
            wrap(&env),
            Err(CodecError::Encode(
                EncodeInvariant::UnsupportedRequiredFeatures { bits: 0b100 }
            ))
        ));
        let mut env = sample(b"x".to_vec());
        env.epoch = 1;
        assert!(matches!(
            wrap(&env),
            Err(CodecError::Encode(EncodeInvariant::UnsupportedEpoch {
                got: 1
            }))
        ));
    }

    #[test]
    fn unknown_optional_feature_is_ignored() {
        let mut env = sample(b"x".to_vec());
        env.optional_features = 0b1111;
        let bytes = wrap(&env).unwrap();
        assert_eq!(unwrap(&bytes).unwrap(), env);
    }

    #[test]
    fn nonzero_epoch_is_fenced() {
        let bytes = forge(0, 1, PayloadKind::ChangesetBundle as u8, 0, b"x", None);
        assert!(matches!(
            unwrap(&bytes),
            Err(CodecError::Fenced(Fenced::Epoch { got: 1 }))
        ));
    }

    /// wrap()으로는 만들 수 없는 형태(미지 kind/flag/required bit/epoch, 조작된 raw_len)를
    /// 유효한 체크섬과 함께 손수 조립한다 — wrap은 라이터 불변식으로 이런 값을 거부하고,
    /// 사후 변조는 체크섬에 걸리므로.
    fn forge(
        required: u64,
        epoch: u64,
        kind_byte: u8,
        flags: u8,
        stored_body: &[u8],
        raw_len: Option<u64>,
    ) -> Vec<u8> {
        let mut bytes = vec![MAGIC, FORMAT_VERSION];
        crate::varint::write_varint(required, &mut bytes); // required
        crate::varint::write_varint(0, &mut bytes); // optional
        crate::varint::write_varint(epoch, &mut bytes); // epoch
        bytes.push(kind_byte);
        bytes.push(flags);
        crate::varint::write_varint(stored_body.len() as u64, &mut bytes);
        if let Some(raw) = raw_len {
            crate::varint::write_varint(raw, &mut bytes);
        }
        let mut hasher = xxhash_rust::xxh3::Xxh3::new();
        hasher.update(&bytes);
        hasher.update(stored_body);
        bytes.extend_from_slice(&hasher.digest().to_le_bytes());
        bytes.extend_from_slice(stored_body);
        bytes
    }

    #[test]
    fn unknown_payload_kind_is_fenced() {
        let bytes = forge(0, 0, 9, 0, b"x", None);
        let result = unwrap(&bytes);
        assert!(
            matches!(
                result,
                Err(CodecError::Fenced(Fenced::PayloadKind { got: 9 }))
            ),
            "{result:?}"
        );
    }

    #[test]
    fn reserved_flag_bit_is_corruption() {
        let bytes = forge(
            0,
            0,
            PayloadKind::ChangesetBundle as u8,
            0b0000_0010,
            b"x",
            None,
        );
        assert!(matches!(
            unwrap(&bytes),
            Err(CodecError::Corruption(Corruption::ReservedFlagBits {
                got: 0b0000_0010
            }))
        ));
    }

    #[test]
    fn required_fence_takes_priority_over_reserved_flags() {
        // v-next 데이터(required bit + 그에 동반된 새 flag)는 Corruption이 아니라
        // Fenced로 분류되어야 한다 — "데이터는 정상, 리더가 낡음"
        let bytes = forge(
            0b100,
            0,
            PayloadKind::ChangesetBundle as u8,
            0b0000_0010,
            b"x",
            None,
        );
        assert!(matches!(
            unwrap(&bytes),
            Err(CodecError::Fenced(Fenced::RequiredFeatures {
                unknown_bits: 0b100
            }))
        ));
    }

    #[test]
    fn compressed_raw_len_bomb_is_rejected_before_allocation() {
        let raw = vec![b'x'; 10_000];
        let compressed = ruzstd::encoding::compress_to_vec(
            raw.as_slice(),
            ruzstd::encoding::CompressionLevel::Fastest,
        );
        let bytes = forge(
            0,
            0,
            PayloadKind::ChangesetBundle as u8,
            FLAG_COMPRESSED,
            &compressed,
            Some(MAX_BODY_BYTES + 1),
        );
        assert!(matches!(
            unwrap(&bytes),
            Err(CodecError::Corruption(Corruption::BodyTooLarge { declared, max }))
                if declared == MAX_BODY_BYTES + 1 && max == MAX_BODY_BYTES
        ));
    }

    #[test]
    fn writer_body_cap_rejects_over_max() {
        // 값 수준 검증이라 거대 버퍼 할당 없이 상한을 테스트한다 (CI OOM 회피)
        assert!(matches!(
            writer_body_cap(MAX_BODY_BYTES + 1),
            Err(CodecError::Encode(EncodeInvariant::BodyTooLarge { .. }))
        ));
        writer_body_cap(MAX_BODY_BYTES).unwrap();
    }

    #[test]
    fn huge_window_declaration_is_rejected_without_allocation() {
        // zstd 프레임 헤더가 2^41 window를 선언(magic + descriptor 0x00 + window 0xF8) —
        // 가드된 reset 경로가 scratch 할당 전에 거부해야 한다
        let bogus_frame = [0x28, 0xB5, 0x2F, 0xFD, 0x00, 0xF8];
        let bytes = forge(
            0,
            0,
            PayloadKind::ChangesetBundle as u8,
            FLAG_COMPRESSED,
            &bogus_frame,
            Some(16),
        );
        assert!(matches!(
            unwrap(&bytes),
            Err(CodecError::Corruption(Corruption::Zstd(_)))
        ));
    }

    #[test]
    fn compressed_trailing_garbage_is_corruption() {
        let raw = vec![b'x'; 10_000];
        let mut stored = ruzstd::encoding::compress_to_vec(
            raw.as_slice(),
            ruzstd::encoding::CompressionLevel::Fastest,
        );
        stored.push(0xEE); // 프레임 뒤 잉여 바이트 — 체크섬은 유효하게 forge됨
        let bytes = forge(
            0,
            0,
            PayloadKind::ChangesetBundle as u8,
            FLAG_COMPRESSED,
            &stored,
            Some(10_000),
        );
        assert!(matches!(
            unwrap(&bytes),
            Err(CodecError::Corruption(Corruption::TrailingBytes {
                remaining: 1
            }))
        ));
    }

    #[test]
    fn uncompressed_body_len_over_cap_is_rejected() {
        // body_len 상한은 헤더 파스 시점에 걸린다 — 체크섬/본문에 도달하기 전
        let mut bytes = vec![MAGIC, FORMAT_VERSION];
        crate::varint::write_varint(0, &mut bytes); // required
        crate::varint::write_varint(0, &mut bytes); // optional
        crate::varint::write_varint(0, &mut bytes); // epoch
        bytes.push(PayloadKind::ChangesetBundle as u8);
        bytes.push(0); // flags
        crate::varint::write_varint(MAX_BODY_BYTES + 1, &mut bytes);
        assert!(matches!(
            unwrap(&bytes),
            Err(CodecError::Corruption(Corruption::BodyTooLarge { .. }))
        ));
    }

    #[test]
    fn compressed_raw_len_mismatch_is_corruption() {
        let raw = vec![b'x'; 10_000];
        let compressed = ruzstd::encoding::compress_to_vec(
            raw.as_slice(),
            ruzstd::encoding::CompressionLevel::Fastest,
        );
        let bytes = forge(
            0,
            0,
            PayloadKind::ChangesetBundle as u8,
            FLAG_COMPRESSED,
            &compressed,
            Some(9_999), // 실제 10_000과 불일치
        );
        assert!(matches!(
            unwrap(&bytes),
            Err(CodecError::Corruption(Corruption::RawLenMismatch {
                declared: 9_999,
                ..
            }))
        ));
    }

    #[test]
    fn body_bit_flip_is_checksum_corruption() {
        let bytes = wrap(&sample(b"checksum-me".to_vec())).unwrap();
        let last = bytes.len() - 1;
        let mut flipped = bytes.clone();
        flipped[last] ^= 0x01;
        assert!(matches!(
            unwrap(&flipped),
            Err(CodecError::Corruption(Corruption::ChecksumMismatch))
        ));
    }

    #[test]
    fn trailing_bytes_rejected_by_unwrap() {
        let mut bytes = wrap(&sample(b"x".to_vec())).unwrap();
        bytes.push(0xEE);
        assert!(matches!(
            unwrap(&bytes),
            Err(CodecError::Corruption(Corruption::TrailingBytes {
                remaining: 1
            }))
        ));
    }

    proptest! {
        #[test]
        fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            let _ = unwrap(&bytes);
        }

        #[test]
        fn round_trip_arbitrary_body(body in proptest::collection::vec(any::<u8>(), 0..2048)) {
            let env = sample(body);
            prop_assert_eq!(unwrap(&wrap(&env).unwrap()).unwrap(), env);
        }
    }
}
