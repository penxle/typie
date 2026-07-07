use editor_codec::CodecError;
use editor_codec::ctx::{read_dot, read_preamble};
use editor_codec::envelope::{self, Envelope, PayloadKind};
use editor_codec::framing::{FrameReader, read_open_variant};
use editor_codec::primitives::{read_char, read_len_prefixed, read_string, read_vec};
use editor_codec::varint::read_varint;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    // 임의 바이트: panic 없이 Ok 또는 분류된 에러만
    #[test]
    fn envelope_decode_total(bytes in proptest::collection::vec(any::<u8>(), 0..1024)) {
        match envelope::unwrap(&bytes) {
            Ok(_) | Err(CodecError::Corruption(_)) | Err(CodecError::Fenced(_)) => {}
            Err(CodecError::Encode(e)) => prop_assert!(false, "decode가 Encode 에러 반환: {e:?}"),
        }
    }

    // 정상 envelope의 임의 1바이트 변조: 체크섬이 checksum 필드 제외 전체를 커버하므로
    // 변조는 Err로 걸린다 — 펜싱 필드(version/required) 변조는 Fenced로, 그 외는
    // Corruption 계열로. 주의: XXH3-64는 비암호 해시라 이것은 2^-64 충돌 확률을 가진
    // 스모크 속성이지 전수 증명이 아니다. proptest 실패 = 커버리지 구멍의 신호.
    #[test]
    fn single_byte_mutation_is_never_silent(
        body in proptest::collection::vec(any::<u8>(), 1..512),
        pos_seed in any::<usize>(),
        xor in 1u8..=255,
    ) {
        let env = Envelope::new(PayloadKind::ChangesetBundle, body);
        let mut bytes = envelope::wrap(&env).unwrap();
        let pos = pos_seed % bytes.len();
        bytes[pos] ^= xor;
        match envelope::unwrap(&bytes) {
            Ok(_) => prop_assert!(false, "1바이트 변조가 조용히 통과 (pos={pos}, xor={xor:#x})"),
            Err(CodecError::Corruption(_)) | Err(CodecError::Fenced(_)) => {}
            Err(CodecError::Encode(e)) => prop_assert!(false, "decode가 Encode 에러 반환: {e:?}"),
        }
    }

    // 하위 디코더들도 동일한 삼분법: 임의 바이트에 panic 없음, Encode 계열 없음
    #[test]
    fn primitives_decode_total(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let mut s = &bytes[..];
        prop_assert!(!matches!(read_string(&mut s), Err(CodecError::Encode(_))));
        let mut s = &bytes[..];
        prop_assert!(!matches!(read_len_prefixed(&mut s), Err(CodecError::Encode(_))));
        let mut s = &bytes[..];
        prop_assert!(!matches!(read_char(&mut s), Err(CodecError::Encode(_))));
        let mut s = &bytes[..];
        prop_assert!(!matches!(read_vec(&mut s, read_varint), Err(CodecError::Encode(_))));
    }

    #[test]
    fn ctx_decode_total(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let mut s = &bytes[..];
        if let Ok(dc) = read_preamble(&mut s) {
            prop_assert!(!matches!(read_dot(&mut s, &dc), Err(CodecError::Encode(_))));
        }
    }

    #[test]
    fn framing_decode_total(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let mut s = &bytes[..];
        prop_assert!(!matches!(read_open_variant(&mut s), Err(CodecError::Encode(_))));
        let mut s = &bytes[..];
        if let Ok(mut frame) = FrameReader::open(&mut s) {
            prop_assert!(!matches!(frame.try_field(read_varint), Err(CodecError::Encode(_))));
        }
    }

    // 번들 디코더로 확장 (Step 4): 공개 changeset-decode 표면도 임의 바이트에
    // panic 없이 분류된 에러(또는 Ok)만 반환해야 한다.
    #[test]
    fn decode_changesets_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        match editor_codec::decode_changesets(&bytes).map(|d| d.into_graph_input()) {
            Ok(_) => {}
            Err(editor_codec::CodecError::Corruption(_)) => {}
            Err(editor_codec::CodecError::Fenced(_)) => {}
            Err(editor_codec::CodecError::Encode(e)) => {
                panic!("디코더가 encode 에러를 반환: {e:?}")
            }
        }
    }

    #[test]
    fn bit_flip_is_detected(flip_at in 0usize..64) {
        let css = vec![]; // 빈 번들 — 가장 작은 유효 envelope
        let mut bytes =
            editor_codec::encode_changesets(editor_codec::ReencodableChangesets::from_local_ops(css)).unwrap();
        let idx = flip_at % bytes.len();
        bytes[idx] ^= 0x01;
        // magic/버전/required 필드의 rot은 Fenced 또는 Corruption으로 오분류될 수 있으나
        // (스펙의 승인된 성질), Ok로 조용히 통과하는 것만은 불허 — 단 checksum 필드 자신의
        // 플립은 Corruption이어야 한다.
        assert!(editor_codec::decode_changesets(&bytes).is_err(), "비트 플립이 조용히 통과");
    }
}

/// 펜싱 원자성: bundle 경유 1케이스. required feature bit이 켜진 envelope를 손조립하면
/// `decode_changesets`가 `Fenced`로 분류되고, 이 시점엔 그래프 적용이 아예 시작되지
/// 않는다(디코드가 원자 단위) — Plan 1 envelope 테스트가 이미 다양한 Fenced 분류를
/// 커버하므로 여기서는 changeset-decode 공개 표면 1케이스만 추가한다.
#[test]
fn bundle_level_required_feature_is_fenced_before_any_graph_application() {
    let mut bytes = vec![
        editor_codec::envelope::MAGIC,
        editor_codec::envelope::FORMAT_VERSION,
    ];
    editor_codec::varint::write_varint(0b1000, &mut bytes); // unknown required feature bit
    editor_codec::varint::write_varint(0, &mut bytes); // optional
    editor_codec::varint::write_varint(0, &mut bytes); // epoch
    bytes.push(editor_codec::envelope::PayloadKind::ChangesetBundle as u8);
    bytes.push(0); // flags
    editor_codec::varint::write_varint(0, &mut bytes); // body_len
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    hasher.update(&bytes);
    bytes.extend_from_slice(&hasher.digest().to_le_bytes());

    assert!(matches!(
        editor_codec::decode_changesets(&bytes),
        Err(editor_codec::CodecError::Fenced(
            editor_codec::Fenced::RequiredFeatures {
                unknown_bits: 0b1000
            }
        ))
    ));
}
