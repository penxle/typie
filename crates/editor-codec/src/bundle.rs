use editor_crdt::Dot;

use crate::ctx::{CollectCtx, DecCtx, EncCtx, read_dot, read_preamble, write_dot, write_preamble};
use crate::durable::Durable;
use crate::envelope::{Envelope, PayloadKind, unwrap_one, wrap};
use crate::error::{CodecResult, Corruption, EncodeInvariant};
use crate::framing::{FrameReader, expect_consumed, write_frame};
use crate::primitives::{read_u8, write_u8};
use crate::types::op::DurableOp;
use crate::varint::{read_varint, write_varint};

#[derive(Debug, Clone, PartialEq)]
pub enum RecordPayload {
    Known(DurableOp),
    Preserved(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BundleRecord {
    pub id: Dot,
    pub parents: Vec<Dot>,
    pub payload: RecordPayload,
    pub record_tail: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BundleChangeset {
    pub records: Vec<BundleRecord>,
}

const MARKER_GENESIS: u8 = 0;
const MARKER_IMPLICIT: u8 = 1;
const MARKER_EXPLICIT: u8 = 2;

/// Observes every dot reachable from `css` (record id/parents, plus whatever a
/// `Known` payload's own fields hold) — non-sealing: it never rejects `Preserved`
/// records or unknown-bearing `Known` payloads, it just builds the actor table.
/// Shared by `collect_bundle` (the sealed, public-encode-facing walk) and the
/// bypass-seal test helper below so both observe dots identically; a walk that
/// skips a `Known` op's own `collect()` under-populates the actor table and any
/// internal `Dot` field (e.g. `SeqUndel`'s `del`, `AddSpan`'s anchors) then fails
/// `write_dot` with `ActorNotCollected` during encode.
fn observe_bundle(css: &[BundleChangeset], cc: &mut CollectCtx) {
    for cs in css {
        for r in &cs.records {
            cc.observe(&r.id);
            for p in &r.parents {
                cc.observe(p);
            }
            if let RecordPayload::Known(op) = &r.payload {
                op.collect(cc);
            }
        }
    }
}

fn collect_bundle(css: &[BundleChangeset], cc: &mut CollectCtx) -> CodecResult<()> {
    observe_bundle(css, cc);
    for cs in css {
        for r in &cs.records {
            match &r.payload {
                RecordPayload::Known(op) => {
                    if op.contains_ctx_unknown() || !r.record_tail.is_empty() {
                        return Err(EncodeInvariant::UnknownPayloadEncode.into());
                    }
                }
                RecordPayload::Preserved(_) => {
                    return Err(EncodeInvariant::UnknownPayloadEncode.into());
                }
            }
        }
    }
    Ok(())
}

fn write_parents(
    parents: &[Dot],
    implicit: Option<Dot>,
    ctx: &EncCtx,
    out: &mut Vec<u8>,
) -> CodecResult<()> {
    if parents.is_empty() {
        write_u8(MARKER_GENESIS, out);
        return Ok(());
    }
    if let Some(prev) = implicit
        && parents.len() == 1
        && parents[0] == prev
    {
        write_u8(MARKER_IMPLICIT, out);
        return Ok(());
    }
    for w in parents.windows(2) {
        if w[0] >= w[1] {
            return Err(EncodeInvariant::NonCanonicalParents.into());
        }
    }
    write_u8(MARKER_EXPLICIT, out);
    write_varint(parents.len() as u64, out);
    for p in parents {
        write_dot(p, ctx, out)?;
    }
    Ok(())
}

fn encode_bundle_body(css: &[BundleChangeset], ctx: &EncCtx, out: &mut Vec<u8>) -> CodecResult<()> {
    write_varint(css.len() as u64, out);
    let mut prev_cs_last: Option<Dot> = None;
    for cs in css {
        if cs.records.is_empty() {
            return Err(EncodeInvariant::EmptyChangeset.into());
        }
        let first = &cs.records[0];
        write_parents(&first.parents, prev_cs_last, ctx, out)?;
        write_varint(cs.records.len() as u64, out);
        let mut prev_op: Option<Dot> = None;
        for (i, r) in cs.records.iter().enumerate() {
            write_frame(out, |body| {
                write_dot(&r.id, ctx, body)?;
                if i > 0 {
                    write_parents(&r.parents, prev_op, ctx, body)?;
                }
                match &r.payload {
                    RecordPayload::Known(op) => {
                        op.encode(ctx, body)?;
                        body.extend_from_slice(&r.record_tail);
                        Ok(())
                    }
                    RecordPayload::Preserved(bytes) => {
                        body.extend_from_slice(bytes);
                        Ok(())
                    }
                }
            })?;
            prev_op = Some(r.id);
        }
        prev_cs_last = prev_op;
    }
    Ok(())
}

pub fn encode_bundle(css: &[BundleChangeset]) -> CodecResult<Vec<u8>> {
    let mut cc = CollectCtx::new();
    collect_bundle(css, &mut cc)?;
    let (actors, baselines) = cc.finalize();
    let ctx = EncCtx::from_parts(&actors, baselines.clone())?;
    let mut body = Vec::new();
    write_preamble(&actors, &baselines, &mut body)?;
    encode_bundle_body(css, &ctx, &mut body)?;
    wrap(&Envelope::new(PayloadKind::ChangesetBundle, body))
}

fn read_parents(input: &mut &[u8], implicit: Option<Dot>, ctx: &DecCtx) -> CodecResult<Vec<Dot>> {
    let marker = read_u8(input)?;
    match marker {
        MARKER_GENESIS => Ok(Vec::new()),
        MARKER_IMPLICIT => match implicit {
            Some(prev) => Ok(vec![prev]),
            None => Err(Corruption::ImplicitPrevWithoutPredecessor.into()),
        },
        MARKER_EXPLICIT => {
            let count = read_varint(input)?;
            if count == 0 {
                return Err(Corruption::EmptyExplicitParents.into());
            }
            let mut parents = Vec::with_capacity((count as usize).min(input.len()));
            for _ in 0..count {
                parents.push(read_dot(input, ctx)?);
            }
            if let Some(prev) = implicit
                && parents.len() == 1
                && parents[0] == prev
            {
                return Err(Corruption::NonCanonicalParentsMarker.into());
            }
            for w in parents.windows(2) {
                if w[0] >= w[1] {
                    return Err(Corruption::NonCanonicalParentsMarker.into());
                }
            }
            Ok(parents)
        }
        _ => Err(Corruption::InvalidParentsMarker { marker }.into()),
    }
}

fn decode_record(
    input: &mut &[u8],
    first: bool,
    cs_parents: &[Dot],
    prev_op: Option<Dot>,
    ctx: &DecCtx,
) -> CodecResult<BundleRecord> {
    let mut frame = FrameReader::open(input)?;
    let id: Dot = frame
        .try_field(|i| read_dot(i, ctx))?
        .ok_or(Corruption::MissingRecordField { field: "id" })?;
    let parents = if first {
        cs_parents.to_vec()
    } else {
        frame
            .try_field(|i| read_parents(i, prev_op, ctx))?
            .ok_or(Corruption::MissingRecordField { field: "parents" })?
    };
    let payload_and_tail = frame.capture_tail();
    if payload_and_tail.is_empty() {
        return Err(Corruption::MissingRecordField { field: "payload" }.into());
    }
    let mut payload_slice = &payload_and_tail[..];
    let op = DurableOp::decode(ctx, &mut payload_slice)?;
    let record_tail = payload_slice.to_vec();
    let (payload, record_tail) = if matches!(op, DurableOp::Unknown(_)) {
        (RecordPayload::Preserved(payload_and_tail), Vec::new())
    } else {
        (RecordPayload::Known(op), record_tail)
    };
    Ok(BundleRecord {
        id,
        parents,
        payload,
        record_tail,
    })
}

fn decode_bundle_changesets(input: &mut &[u8], ctx: &DecCtx) -> CodecResult<Vec<BundleChangeset>> {
    let cs_count = read_varint(input)?;
    let mut css = Vec::new();
    let mut prev_cs_last: Option<Dot> = None;
    for _ in 0..cs_count {
        let cs_parents = read_parents(input, prev_cs_last, ctx)?;
        let op_count = read_varint(input)?;
        if op_count == 0 {
            return Err(Corruption::EmptyChangesetOps.into());
        }
        let mut records = Vec::with_capacity((op_count as usize).min(input.len()));
        let mut prev_op: Option<Dot> = None;
        for i in 0..op_count {
            let r = decode_record(input, i == 0, &cs_parents, prev_op, ctx)?;
            prev_op = Some(r.id);
            records.push(r);
        }
        prev_cs_last = prev_op;
        css.push(BundleChangeset { records });
    }
    Ok(css)
}

fn decode_bundle_body_with_ctx(input: &mut &[u8]) -> CodecResult<(DecCtx, Vec<BundleChangeset>)> {
    let ctx = read_preamble(input)?;
    let css = decode_bundle_changesets(input, &ctx)?;
    expect_consumed(input)?;
    Ok((ctx, css))
}

pub fn decode_bundle(bytes: &[u8]) -> CodecResult<Vec<BundleChangeset>> {
    decode_bundle_with_ctx(bytes).map(|(_, css)| css)
}

/// Body-level decode of an **already-parsed** envelope — for callers that got
/// `envelope` from their own `unwrap`/`unwrap_one` call and would otherwise
/// re-parse the same header a second time via `decode_bundle_with_ctx(bytes)`
/// (e.g. a stream walker that needs the envelope boundary before it can decode).
pub(crate) fn decode_bundle_from_envelope(
    envelope: &Envelope,
) -> CodecResult<(DecCtx, Vec<BundleChangeset>)> {
    if envelope.payload_kind != PayloadKind::ChangesetBundle {
        return Err(Corruption::UnexpectedPayloadKind {
            kind: envelope.payload_kind as u8,
        }
        .into());
    }
    let mut body = &envelope.body[..];
    decode_bundle_body_with_ctx(&mut body)
}

pub(crate) fn decode_bundle_with_ctx(bytes: &[u8]) -> CodecResult<(DecCtx, Vec<BundleChangeset>)> {
    let envelope = crate::envelope::unwrap(bytes)?;
    decode_bundle_from_envelope(&envelope)
}

pub fn decode_bundle_stream(bytes: &[u8]) -> CodecResult<Vec<BundleChangeset>> {
    let mut input = bytes;
    let mut all = Vec::new();
    while !input.is_empty() {
        let envelope = unwrap_one(&mut input)?;
        if envelope.payload_kind != PayloadKind::ChangesetBundle {
            return Err(Corruption::UnexpectedPayloadKind {
                kind: envelope.payload_kind as u8,
            }
            .into());
        }
        let mut body = &envelope.body[..];
        all.extend(decode_bundle_body_with_ctx(&mut body)?.1);
    }
    Ok(all)
}

/// durable **값** 수준의 unknown(예: `Preserved` 레코드, 비어있지 않은 `record_tail`,
/// `Known(op).contains_ctx_unknown()`)만 보는 저렴한 사전 필터 — 변환-결과 손실(런타임
/// 경유 시에만 드러나는 kind-mismatch/중복 드롭 등)은 잡지 않는다. 그 규범 게이트는
/// `convert::Decoded::into_reencodable()`.
pub fn bundle_contains_unknown(bytes: &[u8]) -> CodecResult<bool> {
    if bytes.is_empty() {
        return Ok(false);
    }
    let (_, css) = decode_bundle_with_ctx(bytes)?;
    Ok(css.iter().any(|cs| {
        cs.records.iter().any(|r| match &r.payload {
            RecordPayload::Preserved(_) => true,
            RecordPayload::Known(op) => op.contains_ctx_unknown() || !r.record_tail.is_empty(),
        })
    }))
}

/// `bundle_contains_unknown`의 스트림 판 — 연접된 envelope 각각에 단일-envelope
/// 판을 적용하고 OR로 접는다. `bundle_contains_unknown` 자신은 trailing bytes를
/// 거부하는 단일-envelope 전용이라(envelope.rs:242 `unwrap`) 연접 스트림 입력에
/// 오탐(Corruption::TrailingBytes)한다.
pub fn bundle_stream_contains_unknown(bytes: &[u8]) -> CodecResult<bool> {
    let mut input = bytes;
    let mut any = false;
    while !input.is_empty() {
        let envelope = unwrap_one(&mut input)?;
        if envelope.payload_kind != PayloadKind::ChangesetBundle {
            return Err(Corruption::UnexpectedPayloadKind {
                kind: envelope.payload_kind as u8,
            }
            .into());
        }
        let mut body = &envelope.body[..];
        let ctx = read_preamble(&mut body)?;
        let css = decode_bundle_changesets(&mut body, &ctx)?;
        expect_consumed(body)?;
        any |= css.iter().any(|cs| {
            cs.records.iter().any(|r| match &r.payload {
                RecordPayload::Preserved(_) => true,
                RecordPayload::Known(op) => op.contains_ctx_unknown() || !r.record_tail.is_empty(),
            })
        });
    }
    Ok(any)
}

/// 값 재인코드 없이 changeset 단위로 쪼갠다 — 각 산출은 원본 preamble을 그대로
/// 복사하고 해당 changeset만 다시 프레이밍한다. 첫(유일) changeset이 되므로
/// cs_parents의 Implicit은 write_parents가 자동으로 Explicit으로 승격한다
/// (prev_cs_last가 항상 None으로 시작하기 때문).
pub fn split_bundle_bytes(bytes: &[u8]) -> CodecResult<Vec<Vec<u8>>> {
    let mut input = bytes;
    let mut outputs = Vec::new();
    while !input.is_empty() {
        let envelope = unwrap_one(&mut input)?;
        if envelope.payload_kind != PayloadKind::ChangesetBundle {
            return Err(Corruption::UnexpectedPayloadKind {
                kind: envelope.payload_kind as u8,
            }
            .into());
        }
        let mut body = &envelope.body[..];
        let before = body.len();
        let ctx = read_preamble(&mut body)?;
        let preamble_bytes = &envelope.body[..before - body.len()];
        let css = decode_bundle_changesets(&mut body, &ctx)?;
        expect_consumed(body)?;

        let enc_ctx = EncCtx::from_parts(&ctx.actors, ctx.baselines.clone())?;
        for cs in &css {
            let mut out_body = preamble_bytes.to_vec();
            encode_bundle_body(std::slice::from_ref(cs), &enc_ctx, &mut out_body)?;
            outputs.push(wrap(&Envelope::new(
                PayloadKind::ChangesetBundle,
                out_body,
            ))?);
        }
    }
    Ok(outputs)
}

pub fn encode_dots(dots: &[Dot]) -> CodecResult<Vec<u8>> {
    let mut cc = CollectCtx::new();
    for d in dots {
        cc.observe(d);
    }
    let (actors, baselines) = cc.finalize();
    let ctx = EncCtx::from_parts(&actors, baselines.clone())?;
    let mut body = Vec::new();
    write_preamble(&actors, &baselines, &mut body)?;
    write_varint(dots.len() as u64, &mut body);
    for d in dots {
        write_dot(d, &ctx, &mut body)?;
    }
    wrap(&Envelope::new(PayloadKind::Dots, body))
}

pub fn decode_dots(bytes: &[u8]) -> CodecResult<Vec<Dot>> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let envelope = crate::envelope::unwrap(bytes)?;
    if envelope.payload_kind != PayloadKind::Dots {
        return Err(Corruption::UnexpectedPayloadKind {
            kind: envelope.payload_kind as u8,
        }
        .into());
    }
    let mut input = &envelope.body[..];
    let ctx = read_preamble(&mut input)?;
    let count = read_varint(&mut input)?;
    let mut dots = Vec::with_capacity((count as usize).min(input.len()));
    for _ in 0..count {
        dots.push(read_dot(&mut input, &ctx)?);
    }
    expect_consumed(input)?;
    Ok(dots)
}

#[cfg(feature = "test-util")]
pub fn reencode_for_test(css: &[BundleChangeset], dec: &DecCtx) -> CodecResult<Vec<u8>> {
    let ctx = EncCtx::from_parts(&dec.actors, dec.baselines.clone())?;
    let mut out = Vec::new();
    encode_bundle_body(css, &ctx, &mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodecError;
    use crate::framing::UnknownPayload;
    use crate::types::item::DurableItem;

    fn rec(actor: u64, clock: u64, parents: Vec<Dot>, ch: char) -> BundleRecord {
        BundleRecord {
            id: Dot::new(actor, clock),
            parents,
            payload: RecordPayload::Known(DurableOp::SeqIns {
                pos: 0,
                item: DurableItem::Char(ch),
            }),
            record_tail: Vec::new(),
        }
    }

    #[test]
    fn bundle_round_trips_chain_and_merge() {
        let css = vec![
            BundleChangeset {
                records: vec![rec(1, 0, vec![], 'a'), rec(1, 1, vec![Dot::new(1, 0)], 'b')],
            },
            BundleChangeset {
                records: vec![rec(1, 2, vec![Dot::new(1, 1)], 'c')],
            },
            BundleChangeset {
                records: vec![rec(2, 0, vec![Dot::new(1, 0), Dot::new(1, 2)], 'd')],
            },
        ];
        let bytes = encode_bundle(&css).unwrap();
        assert_eq!(decode_bundle(&bytes).unwrap(), css);
    }

    #[test]
    fn non_first_op_with_empty_parents_round_trips() {
        let css = vec![BundleChangeset {
            records: vec![rec(1, 0, vec![], 'a'), rec(1, 1, vec![], 'b')],
        }];
        let bytes = encode_bundle(&css).unwrap();
        assert_eq!(
            decode_bundle(&bytes).unwrap(),
            css,
            "changeset 내부의 빈-parents op도 전사 가능해야 한다"
        );
    }

    #[test]
    fn parallel_genesis_is_expressible() {
        let css = vec![
            BundleChangeset {
                records: vec![rec(1, 0, vec![], 'a')],
            },
            BundleChangeset {
                records: vec![rec(2, 0, vec![], 'b')],
            },
        ];
        let bytes = encode_bundle(&css).unwrap();
        let decoded = decode_bundle(&bytes).unwrap();
        assert!(
            decoded[1].records[0].parents.is_empty(),
            "두 번째 changeset의 genesis가 보존돼야 한다"
        );
    }

    #[test]
    fn implicit_prev_on_first_changeset_is_corruption() {
        let css = vec![BundleChangeset {
            records: vec![rec(1, 0, vec![], 'a')],
        }];
        let bytes = encode_bundle(&css).unwrap();
        let envelope = crate::envelope::unwrap(&bytes).unwrap();
        let mut body = envelope.body.clone();
        let cs_count_end = {
            let mut probe = &body[..];
            let _ = read_preamble(&mut probe).unwrap();
            let before = probe.len();
            let _ = read_varint(&mut probe).unwrap();
            body.len() - before + (before - probe.len())
        };
        body[cs_count_end] = MARKER_IMPLICIT;
        let mut input = &body[..];
        assert!(matches!(
            decode_bundle_body_with_ctx(&mut input).map(|(_, css)| css),
            Err(CodecError::Corruption(
                Corruption::ImplicitPrevWithoutPredecessor
            ))
        ));
    }

    #[test]
    fn writer_rejects_non_canonical_parents() {
        let css = vec![BundleChangeset {
            records: vec![
                rec(1, 0, vec![], 'a'),
                rec(1, 1, vec![], 'b'),
                rec(1, 2, vec![Dot::new(1, 1), Dot::new(1, 0)], 'c'),
            ],
        }];
        assert!(matches!(
            encode_bundle(&css),
            Err(CodecError::Encode(EncodeInvariant::NonCanonicalParents))
        ));
    }

    #[test]
    fn non_canonical_explicit_parents_is_corruption() {
        // Implicit([prev])과 동일한 값을 Explicit으로 표기한 입력을 수용하면
        // 재인코드 항등이 깨진다 — 리더가 거부해야 한다.
        let actors = [1u64];
        let baselines = vec![0u64];
        let ctx = EncCtx::from_parts(&actors, baselines.clone()).unwrap();
        let mut body = Vec::new();
        write_preamble(&actors, &baselines, &mut body).unwrap();
        write_varint(1, &mut body);
        write_u8(MARKER_GENESIS, &mut body);
        write_varint(2, &mut body);
        write_frame(&mut body, |b| {
            write_dot(&Dot::new(1, 0), &ctx, b)?;
            DurableOp::SeqIns {
                pos: 0,
                item: DurableItem::Char('a'),
            }
            .encode(&ctx, b)
        })
        .unwrap();
        write_frame(&mut body, |b| {
            write_dot(&Dot::new(1, 1), &ctx, b)?;
            write_u8(MARKER_EXPLICIT, b);
            write_varint(1, b);
            write_dot(&Dot::new(1, 0), &ctx, b)?;
            DurableOp::SeqIns {
                pos: 1,
                item: DurableItem::Char('b'),
            }
            .encode(&ctx, b)
        })
        .unwrap();
        let mut input = &body[..];
        assert!(matches!(
            decode_bundle_body_with_ctx(&mut input),
            Err(CodecError::Corruption(
                Corruption::NonCanonicalParentsMarker
            ))
        ));
    }

    #[test]
    fn preserved_payload_rejected_by_public_encode() {
        let css = vec![BundleChangeset {
            records: vec![BundleRecord {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: RecordPayload::Preserved(vec![0x7F, 0x01, 0xAA]),
                record_tail: Vec::new(),
            }],
        }];
        assert!(matches!(
            encode_bundle(&css),
            Err(CodecError::Encode(EncodeInvariant::UnknownPayloadEncode))
        ));
    }

    #[test]
    fn empty_bundle_round_trips() {
        let bytes = encode_bundle(&[]).unwrap();
        assert_eq!(decode_bundle(&bytes).unwrap(), vec![]);
    }

    #[test]
    fn encode_bundle_rejects_empty_changeset() {
        let css = vec![BundleChangeset { records: vec![] }];
        assert!(matches!(
            encode_bundle(&css),
            Err(CodecError::Encode(EncodeInvariant::EmptyChangeset))
        ));
    }

    #[test]
    fn decode_bundle_rejects_empty_changeset_ops() {
        let actors = [1u64];
        let baselines = vec![0u64];
        let mut body = Vec::new();
        write_preamble(&actors, &baselines, &mut body).unwrap();
        write_varint(1, &mut body);
        write_u8(MARKER_GENESIS, &mut body);
        write_varint(0, &mut body);
        let mut input = &body[..];
        assert!(matches!(
            decode_bundle_body_with_ctx(&mut input),
            Err(CodecError::Corruption(Corruption::EmptyChangesetOps))
        ));
    }

    /// `encode_bundle`은 (Task 4부터) `collect_bundle`을 통해 unknown-보유 `Known` 페이로드를
    /// 거부한다 — 로컬 push의 값 수준 봉인. 이 테스트가 검증하려는 건 그 봉인이 아니라
    /// 원격에서 이미 수신된 v-next 바이트의 split/decode 무손실성이므로, 봉인을 우회해
    /// 그런 바이트를 직접 합성한다(수신 경로의 저수준 배관 재현 — 공개 encode 계약과 무관).
    fn encode_bundle_bypassing_unknown_seal(css: &[BundleChangeset]) -> Vec<u8> {
        let mut cc = CollectCtx::new();
        observe_bundle(css, &mut cc);
        let (actors, baselines) = cc.finalize();
        let ctx = EncCtx::from_parts(&actors, baselines.clone()).unwrap();
        let mut body = Vec::new();
        write_preamble(&actors, &baselines, &mut body).unwrap();
        encode_bundle_body(css, &ctx, &mut body).unwrap();
        wrap(&Envelope::new(PayloadKind::ChangesetBundle, body)).unwrap()
    }

    #[test]
    fn bypassing_unknown_seal_observes_dots_inside_known_op_fields() {
        // 회귀 핀: `encode_bundle_bypassing_unknown_seal`가 record id/parents만 관찰하고
        // Known 페이로드 자신의 필드(예: SeqUndel의 `del`, AddSpan의 anchor)를 관찰하지
        // 않으면, 그 필드가 다른 actor의 Dot을 담는 순간 actor 테이블에 없어
        // write_dot이 ActorNotCollected로 실패하고 이 헬퍼의 `.unwrap()`이 패닉한다.
        let css = vec![BundleChangeset {
            records: vec![
                rec(1, 0, vec![], 'a'),
                BundleRecord {
                    id: Dot::new(1, 1),
                    parents: vec![Dot::new(1, 0)],
                    payload: RecordPayload::Known(DurableOp::SeqUndel {
                        del: Dot::new(99, 0),
                    }),
                    record_tail: Vec::new(),
                },
            ],
        }];
        let bytes = encode_bundle_bypassing_unknown_seal(&css);
        assert_eq!(decode_bundle(&bytes).unwrap(), css);
    }

    #[test]
    fn split_bundle_bytes_is_lossless_even_with_unknowns() {
        // 3-changeset 번들(가운데는 synth v-next unknown-보유)을 바이트 분할 →
        // 각 산출을 순차 디코드한 연접 == 원본 번들 디코드 (unknown 레코드의
        // Preserved bytes 포함 완전 동등). Implicit cs_parents의 Explicit 승격도 검증
        // (두 번째·세 번째 산출의 첫 changeset parents가 구체 dot로 복원).
        let unknown_record = BundleRecord {
            id: Dot::new(1, 2),
            parents: vec![Dot::new(1, 1)],
            payload: RecordPayload::Known(DurableOp::Unknown(UnknownPayload {
                tag: 9999,
                bytes: vec![0xde, 0xad, 0xbe, 0xef],
            })),
            record_tail: Vec::new(),
        };
        let css = vec![
            BundleChangeset {
                records: vec![rec(1, 0, vec![], 'a'), rec(1, 1, vec![Dot::new(1, 0)], 'b')],
            },
            BundleChangeset {
                records: vec![unknown_record],
            },
            BundleChangeset {
                records: vec![rec(1, 3, vec![Dot::new(1, 2)], 'c')],
            },
        ];
        let bytes = encode_bundle_bypassing_unknown_seal(&css);
        let expected = decode_bundle(&bytes).unwrap();

        let parts = split_bundle_bytes(&bytes).unwrap();
        assert_eq!(
            parts.len(),
            css.len(),
            "changeset 개수만큼 독립 envelope로 쪼개져야 한다"
        );

        let mut reassembled = Vec::new();
        for part in &parts {
            // ② 산출 각각이 독립 유효 envelope
            reassembled.extend(decode_bundle(part).unwrap());
        }
        // ① decode(원본) == 산출들의 decode 연접
        assert_eq!(reassembled, expected);

        assert_eq!(
            reassembled[1].records[0].parents,
            vec![Dot::new(1, 1)],
            "Implicit이 구체 dot로 승격돼야 한다"
        );
        assert_eq!(
            reassembled[2].records[0].parents,
            vec![Dot::new(1, 2)],
            "Implicit이 구체 dot로 승격돼야 한다"
        );
        assert!(matches!(
            reassembled[1].records[0].payload,
            RecordPayload::Preserved(_)
        ));

        for part in &parts {
            assert_eq!(
                split_bundle_bytes(part).unwrap(),
                vec![part.clone()],
                "이미 단일 changeset인 조각은 자신을 split해도 바이트 동일한 자신이어야 한다"
            );
        }

        let mut concatenated = Vec::new();
        for part in &parts {
            concatenated.extend_from_slice(part);
        }
        assert_eq!(
            split_bundle_bytes(&concatenated).unwrap(),
            parts,
            "산출 조각들의 연접을 다시 split하면 바이트 동일한 조각들로 재폐쇄돼야 한다"
        );
    }

    #[test]
    fn decode_bundle_stream_empty_input_is_empty_list() {
        assert_eq!(
            decode_bundle_stream(&[]).unwrap(),
            Vec::<BundleChangeset>::new()
        );
    }

    #[test]
    fn decode_bundle_stream_concatenates_two_bundles() {
        let css_a = vec![BundleChangeset {
            records: vec![rec(1, 0, vec![], 'a')],
        }];
        let css_b = vec![BundleChangeset {
            records: vec![rec(2, 0, vec![], 'b')],
        }];
        let bytes_a = encode_bundle(&css_a).unwrap();
        let bytes_b = encode_bundle(&css_b).unwrap();
        let mut stream = bytes_a.clone();
        stream.extend_from_slice(&bytes_b);

        let decoded = decode_bundle_stream(&stream).unwrap();
        let mut expected = decode_bundle(&bytes_a).unwrap();
        expected.extend(decode_bundle(&bytes_b).unwrap());
        assert_eq!(decoded, expected);
    }

    #[test]
    fn bundle_contains_unknown_empty_input_is_false() {
        assert!(!bundle_contains_unknown(&[]).unwrap());
    }

    #[test]
    fn bundle_contains_unknown_detects_preserved_and_tail() {
        let css = vec![BundleChangeset {
            records: vec![rec(1, 0, vec![], 'a')],
        }];
        let bytes = encode_bundle(&css).unwrap();
        assert!(!bundle_contains_unknown(&bytes).unwrap());

        let unknown_record = BundleRecord {
            id: Dot::new(1, 1),
            parents: vec![Dot::new(1, 0)],
            payload: RecordPayload::Known(DurableOp::Unknown(UnknownPayload {
                tag: 9999,
                bytes: vec![0xde, 0xad],
            })),
            record_tail: Vec::new(),
        };
        let css_with_unknown = vec![BundleChangeset {
            records: vec![rec(1, 0, vec![], 'a'), unknown_record],
        }];
        let bytes_with_unknown = encode_bundle_bypassing_unknown_seal(&css_with_unknown);
        assert!(bundle_contains_unknown(&bytes_with_unknown).unwrap());
    }

    #[test]
    fn bundle_stream_contains_unknown_empty_input_is_false() {
        assert!(!bundle_stream_contains_unknown(&[]).unwrap());
    }

    #[test]
    fn bundle_stream_contains_unknown_detects_second_envelope() {
        let known_css = vec![BundleChangeset {
            records: vec![rec(1, 0, vec![], 'a')],
        }];
        let known_bytes = encode_bundle(&known_css).unwrap();

        let unknown_record = BundleRecord {
            id: Dot::new(2, 0),
            parents: vec![],
            payload: RecordPayload::Known(DurableOp::Unknown(UnknownPayload {
                tag: 9999,
                bytes: vec![0xde, 0xad],
            })),
            record_tail: Vec::new(),
        };
        let unknown_css = vec![BundleChangeset {
            records: vec![unknown_record],
        }];
        let unknown_bytes = encode_bundle_bypassing_unknown_seal(&unknown_css);

        let mut stream = known_bytes;
        stream.extend_from_slice(&unknown_bytes);

        assert!(
            bundle_stream_contains_unknown(&stream).unwrap(),
            "second envelope's unknown-bearing record must be detected across the stream"
        );
        // 단일-envelope 전용 판은 연접 스트림에 trailing bytes로 오탐 거부한다 —
        // 스트림 판이 존재해야 하는 이유(모듈 doc 참조).
        assert!(bundle_contains_unknown(&stream).is_err());
    }

    #[test]
    fn dots_round_trip() {
        let dots = vec![Dot::new(1, 5), Dot::new(9, 0), Dot::new(1, 6)];
        let bytes = encode_dots(&dots).unwrap();
        assert_eq!(decode_dots(&bytes).unwrap(), dots);
    }

    #[test]
    fn empty_dots_bytes_decode_to_empty() {
        // 구 wire의 빈 heads 표면 계약(0바이트 ↔ 빈 목록) 승계 — FFI 특례의 대칭.
        assert_eq!(decode_dots(&[]).unwrap(), Vec::<Dot>::new());
    }
}
