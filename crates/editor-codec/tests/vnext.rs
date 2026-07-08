use editor_codec::bundle::{
    BundleChangeset, BundleRecord, RecordPayload, decode_bundle, encode_bundle, reencode_for_test,
};
use editor_codec::ctx::{EncCtx, read_preamble, write_dot, write_preamble};
use editor_codec::durable::Durable;
use editor_codec::envelope::{Envelope, PayloadKind, wrap};
use editor_codec::framing::{UnknownPayload, UnknownTail, write_frame};
use editor_codec::primitives::{read_len_prefixed, read_u8};
use editor_codec::types::anchor::{DurableAnchor, DurableBias};
use editor_codec::types::attr::DurableAttr;
use editor_codec::types::item::{DurableItem, DurableNodeType};
use editor_codec::types::modifier::DurableModifier;
use editor_codec::types::op::{DurableAliasRun, DurableOp};
use editor_codec::varint::{read_varint, write_varint};
use editor_codec::{CodecError, Fenced};
use editor_crdt::{Dot, ListOp};
use editor_model::{AtomLeaf, EditOp, NodeType, SeqItem};

const VNEXT_OP_TAG: u64 = 999;
const VNEXT_ITEM_TAG: u64 = 998;

/// Preamble-relative decode of the body, returning the slice right after it —
/// mirrors what `decode_bundle_with_ctx`/`reencode_for_test` expect as input.
fn body_after_preamble(body: &[u8]) -> Vec<u8> {
    let mut slice = body;
    read_preamble(&mut slice).unwrap();
    slice.to_vec()
}

/// Synthetic bundle: [known char ins] -> [v-next op (unknown tag 999, payload
/// carries a real preamble-relative Dot)] -> [known ins carrying a v-next item
/// (unknown tag 998)], a single 3-op changeset on one actor.
fn synth_bundle() -> Vec<u8> {
    let actors = [5u64];
    let baselines = [0u64];
    let ctx = EncCtx::from_parts(&actors, baselines.to_vec()).unwrap();
    let mut body = Vec::new();
    write_preamble(&actors, &baselines, &mut body).unwrap();
    write_varint(1, &mut body); // changeset_count
    body.push(0); // cs_parents: Genesis
    write_varint(3, &mut body); // op_count

    // record 0: known SeqIns Char — normal encode path
    write_frame(&mut body, |b| {
        write_dot(&Dot::new(5, 0), &ctx, b)?;
        DurableOp::SeqIns {
            pos: 0,
            item: DurableItem::Char('a'),
        }
        .encode(&ctx, b)
    })
    .unwrap();

    // record 1: v-next op — unknown tag 999, payload = [Dot(5,0) ctx-relative, 0xEE].
    // Constructed via `DurableOp::Unknown`'s own encode (write_unknown_variant),
    // which is byte-identical to hand-writing varint(tag)+varint(len)+payload.
    write_frame(&mut body, |b| {
        write_dot(&Dot::new(5, 1), &ctx, b)?;
        b.push(1); // op_parents marker: Implicit (prev = record 0's id)
        let mut payload = Vec::new();
        write_dot(&Dot::new(5, 0), &ctx, &mut payload)?;
        payload.push(0xEE);
        DurableOp::Unknown(UnknownPayload {
            tag: VNEXT_OP_TAG,
            bytes: payload,
        })
        .encode(&ctx, b)
    })
    .unwrap();

    // record 2: known SeqIns carrying a v-next item (unknown tag 998, payload [0xDD])
    write_frame(&mut body, |b| {
        write_dot(&Dot::new(5, 2), &ctx, b)?;
        b.push(1); // Implicit (prev = record 1's id)
        DurableOp::SeqIns {
            pos: 1, // seq-parent skips record 1 (non-seq); insert after record 0's char
            item: DurableItem::Unknown(UnknownPayload {
                tag: VNEXT_ITEM_TAG,
                bytes: vec![0xDD],
            }),
        }
        .encode(&ctx, b)
    })
    .unwrap();

    let envelope = Envelope::new(PayloadKind::ChangesetBundle, body);
    wrap(&envelope).unwrap()
}

#[test]
fn vnext_bytes_survive_reencode_identically() {
    let bytes = synth_bundle();
    let decoded = decode_bundle(&bytes).unwrap();
    assert!(matches!(
        decoded[0].records[1].payload,
        RecordPayload::Preserved(_)
    ));
    let envelope = editor_codec::envelope::unwrap(&bytes).unwrap();
    let mut body = &envelope.body[..];
    let dec = read_preamble(&mut body).unwrap();
    let reencoded = reencode_for_test(&decoded, &dec).unwrap();
    assert_eq!(
        reencoded,
        body_after_preamble(&envelope.body),
        "v-next 재인코드 바이트 항등"
    );
}

#[test]
fn vnext_item_occupies_one_slot_in_projection() {
    let css = editor_codec::decode_changesets(&synth_bundle())
        .unwrap()
        .into_graph_input();
    let g = editor_crdt::OpGraph::from_changesets(css).unwrap();
    let logs = editor_model::split_logs(&g).unwrap();
    let items = editor_crdt::sequence::checkout(&logs.seq);
    let kinds: Vec<&'static str> = items
        .iter()
        .map(|(_, item)| match item {
            SeqItem::Char(_) => "char",
            SeqItem::Unknown { .. } => "unknown",
            _ => "other",
        })
        .collect();
    assert_eq!(
        kinds,
        vec!["char", "unknown"],
        "v-next item은 SeqItem::Unknown으로 정확히 1 슬롯을 차지해야 한다"
    );
    match &items[1].1 {
        SeqItem::Unknown { tag, bytes } => {
            assert_eq!(*tag, VNEXT_ITEM_TAG);
            assert_eq!(bytes, &vec![0xDD]);
        }
        other => panic!("expected SeqItem::Unknown, got {other:?}"),
    }
}

/// Reads `[cs_count varint][cs_parents marker u8][op_count varint][frame]` and
/// returns the frame's raw body bytes (post length-prefix) — used to compare
/// a single-record segment across two independently-built bundles that share
/// the same preamble/ctx and both have exactly one changeset with Genesis parents.
fn first_record_frame_body(body: &[u8]) -> Vec<u8> {
    let mut s = body;
    let _cs_count = read_varint(&mut s).unwrap();
    let marker = read_u8(&mut s).unwrap();
    assert_eq!(marker, 0, "test fixture always uses Genesis cs_parents");
    let _op_count = read_varint(&mut s).unwrap();
    read_len_prefixed(&mut s).unwrap().to_vec()
}

#[test]
fn known_subset_reencodes_like_normal_encoding() {
    // Unknown 레코드를 뺀 known 부분집합(record 0)의 재인코딩이, 그 op만으로
    // 공개 encode_bundle을 호출한 정상 경로와 바이트 동일해야 한다.
    let bytes = synth_bundle();
    let envelope = editor_codec::envelope::unwrap(&bytes).unwrap();
    let synth_frame = first_record_frame_body(&body_after_preamble(&envelope.body));

    let record0 = BundleRecord {
        id: Dot::new(5, 0),
        parents: vec![],
        payload: RecordPayload::Known(DurableOp::SeqIns {
            pos: 0,
            item: DurableItem::Char('a'),
        }),
        record_tail: Vec::new(),
    };
    let normal_bytes = encode_bundle(&[BundleChangeset {
        records: vec![record0],
    }])
    .unwrap();
    let normal_envelope = editor_codec::envelope::unwrap(&normal_bytes).unwrap();
    let normal_frame = first_record_frame_body(&body_after_preamble(&normal_envelope.body));

    assert_eq!(
        synth_frame, normal_frame,
        "known 부분집합의 재인코딩은 현행 정상 인코딩과 바이트 동일해야 한다"
    );
}

/// 위 오라클은 `DurableOp::SeqIns(Char)` 한 variant만 커버했다 — 그 성질(known
/// 레코드의 프레임 인코딩은 정상 `encode_bundle` 경로와 바이트 동일)이 다른
/// record 계열에도 성립하는지 폭을 넓혀 확인한다.
#[test]
fn known_subset_reencodes_like_normal_encoding_across_op_kinds() {
    let anchor = Dot::new(5, 0);
    let ops = [
        DurableOp::SeqDel { pos: 0, len: 1 },
        DurableOp::AddSpan {
            start: DurableAnchor {
                id: anchor,
                bias: DurableBias::Before,
            },
            end: DurableAnchor {
                id: anchor,
                bias: DurableBias::After,
            },
            modifier: DurableModifier::Bold,
            tail: UnknownTail(Vec::new()),
        },
        DurableOp::SetBlockModifier {
            target: anchor,
            modifier: DurableModifier::Bold,
            tail: UnknownTail(Vec::new()),
        },
        DurableOp::AliasDots {
            pairs: vec![DurableAliasRun {
                old_start: anchor,
                len: 1,
                new_start: anchor,
            }],
            tail: UnknownTail(Vec::new()),
        },
    ];

    for op in ops {
        let solo_bytes = single_op_bundle(&[5], &[0], anchor, &op);
        let solo_envelope = editor_codec::envelope::unwrap(&solo_bytes).unwrap();
        let solo_frame = first_record_frame_body(&body_after_preamble(&solo_envelope.body));

        let record = BundleRecord {
            id: anchor,
            parents: vec![],
            payload: RecordPayload::Known(op.clone()),
            record_tail: Vec::new(),
        };
        let normal_bytes = encode_bundle(&[BundleChangeset {
            records: vec![record],
        }])
        .unwrap();
        let normal_envelope = editor_codec::envelope::unwrap(&normal_bytes).unwrap();
        let normal_frame = first_record_frame_body(&body_after_preamble(&normal_envelope.body));

        assert_eq!(
            solo_frame, normal_frame,
            "{op:?}의 known 부분집합 재인코딩이 정상 인코딩과 달라짐"
        );
    }
}

fn single_op_bundle(actors: &[u64], baselines: &[u64], id: Dot, op: &DurableOp) -> Vec<u8> {
    let ctx = EncCtx::from_parts(actors, baselines.to_vec()).unwrap();
    let mut body = Vec::new();
    write_preamble(actors, baselines, &mut body).unwrap();
    write_varint(1, &mut body); // changeset_count
    body.push(0); // Genesis
    write_varint(1, &mut body); // op_count
    write_frame(&mut body, |b| {
        write_dot(&id, &ctx, b)?;
        op.encode(&ctx, b)
    })
    .unwrap();
    wrap(&Envelope::new(PayloadKind::ChangesetBundle, body)).unwrap()
}

#[test]
fn evolvable_tail_bytes_demote_op_and_durable_reencode_holds() {
    // AddSpan(evolvable payload) 끝에 여분 바이트("미래 필드") — 손인코딩이 아니라
    // 값 자체에 non-empty UnknownTail을 실어 구성한다(동일 wire 효과).
    let anchor = Dot::new(5, 0);
    let op = DurableOp::AddSpan {
        start: DurableAnchor {
            id: anchor,
            bias: DurableBias::Before,
        },
        end: DurableAnchor {
            id: anchor,
            bias: DurableBias::After,
        },
        modifier: DurableModifier::Bold,
        tail: UnknownTail(vec![0xAA, 0xBB]),
    };

    // durable 계층 재인코드 항등
    let ctx = EncCtx::from_parts(&[5], vec![0]).unwrap();
    let mut bytes = Vec::new();
    op.encode(&ctx, &mut bytes).unwrap();
    let dec = editor_codec::ctx::DecCtx {
        actors: vec![5],
        baselines: vec![0],
    };
    let mut slice = &bytes[..];
    let decoded_op = DurableOp::decode(&dec, &mut slice).unwrap();
    assert!(slice.is_empty());
    let mut reencoded = Vec::new();
    decoded_op.encode(&ctx, &mut reencoded).unwrap();
    assert_eq!(reencoded, bytes);

    // op 수준 EditOp::Unknown 강등 (convert 경유, 실 changeset 파이프라인)
    let bundle_bytes = single_op_bundle(&[5], &[0], Dot::new(5, 1), &op);
    let ops = editor_codec::decode_changesets(&bundle_bytes)
        .unwrap()
        .into_graph_input();
    assert!(matches!(ops[0].ops[0].payload, EditOp::Unknown { .. }));

    let lossy_gate = editor_codec::decode_changesets(&bundle_bytes)
        .unwrap()
        .into_reencodable();
    assert!(matches!(
        lossy_gate,
        Err(CodecError::Fenced(Fenced::LossyForReencode))
    ));
}

#[test]
fn alias_dots_tail_bytes_demote_regardless_of_base_validity() {
    // AliasDots(open payload)에 non-empty tail + 그 자체로는 alias_op_is_valid를 통과
    // 못할 base pairs(self-run: old_start == new_start)를 함께 실었다. tail nonempty
    // 감지가 semantic 검증보다 먼저 실행되어야 하므로, 이 op은 Corruption이 아니라
    // EditOp::Unknown으로 강등되어야 한다(v2 확장 의미에서 이 base 값이 정당할 수
    // 있으므로 — open-tail 전방 호환의 직접 핀).
    let dot = Dot::new(5, 0);
    let op = DurableOp::AliasDots {
        pairs: vec![DurableAliasRun {
            old_start: dot,
            len: 1,
            new_start: dot,
        }],
        tail: UnknownTail(vec![0xAA, 0xBB]),
    };

    // durable 계층 재인코드 항등
    let ctx = EncCtx::from_parts(&[5], vec![0]).unwrap();
    let mut bytes = Vec::new();
    op.encode(&ctx, &mut bytes).unwrap();
    let dec = editor_codec::ctx::DecCtx {
        actors: vec![5],
        baselines: vec![0],
    };
    let mut slice = &bytes[..];
    let decoded_op = DurableOp::decode(&dec, &mut slice).unwrap();
    assert!(slice.is_empty());
    let mut reencoded = Vec::new();
    decoded_op.encode(&ctx, &mut reencoded).unwrap();
    assert_eq!(reencoded, bytes);

    // op 수준 EditOp::Unknown 강등 (convert 경유, 실 changeset 파이프라인) — Corruption 아님
    let bundle_bytes = single_op_bundle(&[5], &[0], Dot::new(5, 1), &op);
    let ops = editor_codec::decode_changesets(&bundle_bytes)
        .unwrap()
        .into_graph_input();
    assert!(matches!(ops[0].ops[0].payload, EditOp::Unknown { .. }));

    let lossy_gate = editor_codec::decode_changesets(&bundle_bytes)
        .unwrap()
        .into_reencodable();
    assert!(matches!(
        lossy_gate,
        Err(CodecError::Fenced(Fenced::LossyForReencode))
    ));
}

#[test]
fn alias_dots_invalid_base_without_tail_is_rejected_as_corruption() {
    // tail이 비어 있으면 base validity(①~⑥, alias_op_is_valid와 동치)가 그대로
    // 적용된다: self-run(old_start == new_start)은 원격 경로의 invalid AliasOp로
    // 취급되어 Corruption으로 원자적 디코드 실패해야 한다.
    let dot = Dot::new(5, 0);
    let op = DurableOp::AliasDots {
        pairs: vec![DurableAliasRun {
            old_start: dot,
            len: 1,
            new_start: dot,
        }],
        tail: UnknownTail(Vec::new()),
    };
    let bundle_bytes = single_op_bundle(&[5], &[0], Dot::new(5, 1), &op);
    assert!(matches!(
        editor_codec::decode_changesets(&bundle_bytes),
        Err(CodecError::Corruption(
            editor_codec::Corruption::InvalidAliasOp
        ))
    ));
}

/// 5번째 픽스처 — known Block(Paragraph) + 미래 init attr(미지 태그 99, Dot 없는
/// 값 payload만) + 그 블록을 parent로 하는 자식 char 1개. additive-safe의
/// 직접 타격: 미지 attr가 블록을 opaque 리프로 강등해 자식을 고아화하는 회귀를 잡는다.
fn synth_block_with_unknown_attr_bundle() -> Vec<u8> {
    let actors = [0u64, 5u64];
    let baselines = [0u64, 0u64];
    let ctx = EncCtx::from_parts(&actors, baselines.to_vec()).unwrap();
    let mut body = Vec::new();
    write_preamble(&actors, &baselines, &mut body).unwrap();
    write_varint(1, &mut body); // changeset_count
    body.push(0); // Genesis
    write_varint(2, &mut body); // op_count

    write_frame(&mut body, |b| {
        write_dot(&Dot::new(5, 0), &ctx, b)?;
        DurableOp::SeqIns {
            pos: 0,
            item: DurableItem::Block {
                node_type: DurableNodeType::Paragraph,
                parents: vec![Dot::ROOT],
                init: vec![DurableAttr::Unknown(UnknownPayload {
                    tag: 99,
                    bytes: vec![0x01, 0x02],
                })],
                tail: UnknownTail(Vec::new()),
            },
        }
        .encode(&ctx, b)
    })
    .unwrap();

    write_frame(&mut body, |b| {
        write_dot(&Dot::new(5, 1), &ctx, b)?;
        b.push(1); // Implicit (prev = the block's id)
        DurableOp::SeqIns {
            pos: 1,
            item: DurableItem::Char('x'),
        }
        .encode(&ctx, b)
    })
    .unwrap();

    wrap(&Envelope::new(PayloadKind::ChangesetBundle, body)).unwrap()
}

#[test]
fn unknown_init_attr_keeps_block_structure_and_child_attaches() {
    let bytes = synth_block_with_unknown_attr_bundle();

    // ① convert 경유 디코드: SeqItem::Block 구조 유지, attrs에 NodeAttr::Unknown{tag:99,..}
    let decoded = editor_codec::decode_changesets(&bytes)
        .unwrap()
        .into_graph_input();
    let EditOp::Seq(ListOp::Ins {
        item: SeqItem::Block {
            node_type, attrs, ..
        },
        ..
    }) = &decoded[0].ops[0].payload
    else {
        panic!(
            "expected SeqItem::Block, got {:?}",
            decoded[0].ops[0].payload
        );
    };
    assert_eq!(*node_type, NodeType::Paragraph);
    assert!(matches!(
        attrs.as_slice(),
        [editor_model::NodeAttr::Unknown { tag: 99, bytes }] if bytes == &vec![0x01, 0x02]
    ));

    // ② 투영에서 자식 char가 정상 부착(고아 0)
    let g = editor_crdt::OpGraph::from_changesets(decoded).unwrap();
    let pd = editor_model::project_document(&editor_model::split_logs(&g).unwrap()).unwrap();
    let root = pd.tree.root_node().unwrap();
    assert_eq!(root.children.len(), 1);
    let editor_model::Child::Block(para_id) = root.children.get(0).unwrap() else {
        panic!("expected the paragraph block as root's only child");
    };
    let para = pd.tree.get(*para_id).unwrap();
    assert_eq!(para.children.len(), 1);
    assert!(matches!(
        para.children.get(0).unwrap(),
        editor_model::Child::Leaf {
            item: SeqItem::Char('x'),
            ..
        }
    ));

    // ③ durable 계층 재인코드 바이트 항등
    let (dec, bundle_css) = {
        let envelope = editor_codec::envelope::unwrap(&bytes).unwrap();
        let mut b = &envelope.body[..];
        let dec = read_preamble(&mut b).unwrap();
        (dec, decode_bundle(&bytes).unwrap())
    };
    let reencoded = reencode_for_test(&bundle_css, &dec).unwrap();
    let envelope = editor_codec::envelope::unwrap(&bytes).unwrap();
    assert_eq!(reencoded, body_after_preamble(&envelope.body));

    // ④ 런타임 재인코드도 합법: NodeAttr::Unknown은 ctx-독립(봉인 비대상) — 성공하고
    // attr 구간이 원본과 바이트 동일(정준 재방출).
    let decoded_again = editor_codec::decode_changesets(&bytes)
        .unwrap()
        .into_graph_input();
    let reencoded_runtime = editor_codec::encode_changesets(
        editor_codec::ReencodableChangesets::from_local_ops(decoded_again),
    )
    .unwrap();
    let runtime_envelope = editor_codec::envelope::unwrap(&reencoded_runtime).unwrap();
    let mut runtime_body = &runtime_envelope.body[..];
    let runtime_dec = read_preamble(&mut runtime_body).unwrap();
    // actor 5 is present in both original and runtime-reencoded ctx; the attr
    // payload bytes for tag 99 must match byte-for-byte regardless of table shape.
    assert!(runtime_dec.actors.contains(&5));
    // Decode the runtime re-encode again and inspect the structured attr directly
    // (rather than scanning raw bytes for a coincidental [0x01, 0x02] window) —
    // proves the tag-99 payload survived at its actual field, not just somewhere
    // in the stream.
    let decoded_reencoded = editor_codec::decode_changesets(&reencoded_runtime)
        .unwrap()
        .into_graph_input();
    let EditOp::Seq(ListOp::Ins {
        item: SeqItem::Block { attrs, .. },
        ..
    }) = &decoded_reencoded[0].ops[0].payload
    else {
        panic!(
            "expected SeqItem::Block after runtime reencode, got {:?}",
            decoded_reencoded[0].ops[0].payload
        );
    };
    assert!(
        matches!(
            attrs.as_slice(),
            [editor_model::NodeAttr::Unknown { tag: 99, bytes }] if bytes == &vec![0x01, 0x02]
        ),
        "재인코드 산출을 다시 디코드해도 원본 attr payload가 tag/bytes 그대로여야 한다"
    );
}

/// 6번째 픽스처 — known SeqIns(Char) 레코드의 frame 안에서 DurableOp payload 뒤에
/// 여분 바이트(record_tail)를 붙인다(미래 op 메타데이터 시뮬레이션).
fn synth_seqins_with_record_tail() -> Vec<u8> {
    let actors = [5u64];
    let baselines = [0u64];
    let ctx = EncCtx::from_parts(&actors, baselines.to_vec()).unwrap();
    let mut body = Vec::new();
    write_preamble(&actors, &baselines, &mut body).unwrap();
    write_varint(1, &mut body);
    body.push(0); // Genesis
    write_varint(1, &mut body);
    write_frame(&mut body, |b| {
        write_dot(&Dot::new(5, 0), &ctx, b)?;
        DurableOp::SeqIns {
            pos: 0,
            item: DurableItem::Char('a'),
        }
        .encode(&ctx, b)?;
        b.extend_from_slice(&[0xEE, 0x07]);
        Ok(())
    })
    .unwrap();
    wrap(&Envelope::new(PayloadKind::ChangesetBundle, body)).unwrap()
}

#[test]
fn record_tail_preserves_op_shape_but_gates_encode() {
    let bytes = synth_seqins_with_record_tail();

    // ① decode_bundle이 Known(SeqIns) + record_tail == [0xEE, 0x07]로 분류
    let decoded = decode_bundle(&bytes).unwrap();
    let record = &decoded[0].records[0];
    assert!(matches!(
        record.payload,
        RecordPayload::Known(DurableOp::SeqIns { .. })
    ));
    assert_eq!(record.record_tail, vec![0xEE, 0x07]);

    // ② convert 경유 디코드에서 EditOp::Seq(Ins{..}) 구조 유지 (op-Unknown 강등 아님)
    let ops = editor_codec::decode_changesets(&bytes)
        .unwrap()
        .into_graph_input();
    assert!(matches!(
        ops[0].ops[0].payload,
        EditOp::Seq(ListOp::Ins { .. })
    ));

    // ③ durable 계층 재인코드(reencode_for_test) 바이트 항등 (tail splice)
    let envelope = editor_codec::envelope::unwrap(&bytes).unwrap();
    let mut b = &envelope.body[..];
    let dec = read_preamble(&mut b).unwrap();
    let reencoded = reencode_for_test(&decoded, &dec).unwrap();
    assert_eq!(reencoded, body_after_preamble(&envelope.body));

    // ④ bundle_contains_unknown(bytes) == true
    assert!(editor_codec::bundle_contains_unknown(&bytes).unwrap());

    // ⑤ 공개 encode_bundle은 record_tail이 비어있지 않은 레코드를 EncodeInvariant로 거부
    assert!(matches!(
        encode_bundle(&decoded),
        Err(CodecError::Encode(
            editor_codec::EncodeInvariant::UnknownPayloadEncode
        ))
    ));
}

/// 7번째 픽스처 — known op + 미지 modifier: `SetBlockModifier`의 modifier 필드가
/// 미지 태그(97)인 op을 손인코딩한다. 비-SeqIns 강등 규칙의 직접 핀.
#[test]
fn known_op_with_unknown_modifier_demotes_at_op_level() {
    let op = DurableOp::SetBlockModifier {
        target: Dot::new(5, 0),
        modifier: DurableModifier::Unknown(UnknownPayload {
            tag: 97,
            bytes: vec![0x01],
        }),
        tail: UnknownTail(Vec::new()),
    };
    let bytes = single_op_bundle(&[5], &[0], Dot::new(5, 1), &op);

    // ① convert 경유 디코드가 op 수준 EditOp::Unknown
    let ops = editor_codec::decode_changesets(&bytes)
        .unwrap()
        .into_graph_input();
    assert!(matches!(ops[0].ops[0].payload, EditOp::Unknown { .. }));

    // ② durable 계층 재인코드 항등
    let ctx = EncCtx::from_parts(&[5], vec![0]).unwrap();
    let mut op_bytes = Vec::new();
    op.encode(&ctx, &mut op_bytes).unwrap();
    let dec = editor_codec::ctx::DecCtx {
        actors: vec![5],
        baselines: vec![0],
    };
    let mut slice = &op_bytes[..];
    let decoded_op = DurableOp::decode(&dec, &mut slice).unwrap();
    assert!(slice.is_empty());
    let mut reencoded = Vec::new();
    decoded_op.encode(&ctx, &mut reencoded).unwrap();
    assert_eq!(reencoded, op_bytes);

    // ③ bundle_contains_unknown == true
    assert!(editor_codec::bundle_contains_unknown(&bytes).unwrap());
}

/// 8번째 픽스처 — 서로 다른 actor 집합의 envelope 2개를 연접(두 번째에 미지
/// modifier op 포함).
#[test]
fn multi_preamble_stream_resolves_each_envelope_independently() {
    let known_op = DurableOp::SeqIns {
        pos: 0,
        item: DurableItem::Char('a'),
    };
    let unknown_op = DurableOp::SetBlockModifier {
        target: Dot::new(11, 100),
        modifier: DurableModifier::Unknown(UnknownPayload {
            tag: 97,
            bytes: vec![9],
        }),
        tail: UnknownTail(Vec::new()),
    };

    let env_a = single_op_bundle(&[5], &[0], Dot::new(5, 0), &known_op);
    let env_b = single_op_bundle(&[11], &[100], Dot::new(11, 100), &unknown_op);

    let mut stream = env_a.clone();
    stream.extend_from_slice(&env_b);

    let decoded = editor_codec::decode_changeset_stream(&stream).unwrap();
    let ops = decoded.into_graph_input();
    assert_eq!(ops.len(), 2);

    // ① known op들의 Dot을 각자 preamble 기준으로 정확 해석
    assert_eq!(ops[0].ops[0].id.actor, 5);
    assert_eq!(ops[1].ops[0].id.actor, 11);
    assert_eq!(ops[1].ops[0].id.clock, 100);

    // ② 두 번째 envelope의 미지 modifier op이 EditOp::Unknown으로 수용되고 그 bytes가
    // 자기 envelope의 ctx로 합성되어 있음(개별 decode_changesets 결과와 바이트 일치)
    let EditOp::Unknown {
        bytes: stream_bytes,
    } = &ops[1].ops[0].payload
    else {
        panic!("expected EditOp::Unknown for the second envelope's op");
    };
    let solo = editor_codec::decode_changesets(&env_b)
        .unwrap()
        .into_graph_input();
    let EditOp::Unknown { bytes: solo_bytes } = &solo[0].ops[0].payload else {
        panic!("expected EditOp::Unknown when decoding envelope B alone");
    };
    assert_eq!(stream_bytes, solo_bytes);

    // ③ 스트림 lossless가 AND로 접힘(false)
    let lossy_gate = editor_codec::decode_changeset_stream(&stream)
        .unwrap()
        .into_reencodable();
    assert!(matches!(
        lossy_gate,
        Err(CodecError::Fenced(Fenced::LossyForReencode))
    ));
}

/// 9번째 픽스처 — 미지 node_type Block/BlockAtom -> 자리표시자 (Task 1b 매핑의 직접 핀).
/// 미지 태그(96)의 Block + 그 블록을 parent로 하는 자식 char 1개 + 같은 changeset에
/// 미지 node_type BlockAtom 1개(루트의 direct leaf) + 트레일링 실 Paragraph
/// (Root의 content 요건 `(...)* , Paragraph`을 충족하기 위함 — placeholder들은
/// NodeType::Unknown으로 필터되어 kids 목록에서 보이지 않는다).
fn synth_unknown_node_type_bundle() -> Vec<u8> {
    let actors = [0u64, 5u64];
    let baselines = [0u64, 0u64];
    let ctx = EncCtx::from_parts(&actors, baselines.to_vec()).unwrap();
    let mut body = Vec::new();
    write_preamble(&actors, &baselines, &mut body).unwrap();
    write_varint(1, &mut body);
    body.push(0); // Genesis
    write_varint(5, &mut body); // op_count

    // op0: placeholder Block, unknown node_type tag 96, parent = Root
    write_frame(&mut body, |b| {
        write_dot(&Dot::new(5, 0), &ctx, b)?;
        DurableOp::SeqIns {
            pos: 0,
            item: DurableItem::Block {
                node_type: DurableNodeType::Unknown(UnknownPayload {
                    tag: 96,
                    bytes: vec![0x01],
                }),
                parents: vec![Dot::ROOT],
                init: vec![],
                tail: UnknownTail(Vec::new()),
            },
        }
        .encode(&ctx, b)
    })
    .unwrap();

    // op1: child char of the placeholder block
    write_frame(&mut body, |b| {
        write_dot(&Dot::new(5, 1), &ctx, b)?;
        b.push(1); // Implicit (prev = op0's id)
        DurableOp::SeqIns {
            pos: 1,
            item: DurableItem::Char('x'),
        }
        .encode(&ctx, b)
    })
    .unwrap();

    // op2: placeholder BlockAtom, unknown node_type tag 96, parent = Root
    write_frame(&mut body, |b| {
        write_dot(&Dot::new(5, 2), &ctx, b)?;
        b.push(1); // Implicit (prev = op1's id)
        DurableOp::SeqIns {
            pos: 2,
            item: DurableItem::BlockAtom {
                node_type: DurableNodeType::Unknown(UnknownPayload {
                    tag: 96,
                    bytes: vec![0x02],
                }),
                parents: vec![Dot::ROOT],
                init: vec![],
                tail: UnknownTail(Vec::new()),
            },
        }
        .encode(&ctx, b)
    })
    .unwrap();

    // op3: real trailing Paragraph (Root content requires a trailing Paragraph)
    write_frame(&mut body, |b| {
        write_dot(&Dot::new(5, 3), &ctx, b)?;
        b.push(1); // Implicit (prev = op2's id)
        DurableOp::SeqIns {
            pos: 3,
            item: DurableItem::Block {
                node_type: DurableNodeType::Paragraph,
                parents: vec![Dot::ROOT],
                init: vec![],
                tail: UnknownTail(Vec::new()),
            },
        }
        .encode(&ctx, b)
    })
    .unwrap();

    // op4: char inside the real paragraph
    write_frame(&mut body, |b| {
        write_dot(&Dot::new(5, 4), &ctx, b)?;
        b.push(1); // Implicit (prev = op3's id)
        DurableOp::SeqIns {
            pos: 4,
            item: DurableItem::Char('p'),
        }
        .encode(&ctx, b)
    })
    .unwrap();

    wrap(&Envelope::new(PayloadKind::ChangesetBundle, body)).unwrap()
}

#[test]
fn unknown_node_type_becomes_placeholder_and_stays_attached() {
    let bytes = synth_unknown_node_type_bundle();

    // ① convert 경유 디코드: Block은 SeqItem::Block{node_type:Unknown,..}(구조 유지),
    // BlockAtom은 SeqItem::BlockAtom{leaf:AtomLeaf::Unknown(_),..}(리프 모양 유지)
    let decoded = editor_codec::decode_changesets(&bytes)
        .unwrap()
        .into_graph_input();
    assert!(matches!(
        &decoded[0].ops[0].payload,
        EditOp::Seq(ListOp::Ins {
            item: SeqItem::Block {
                node_type: NodeType::Unknown,
                ..
            },
            ..
        })
    ));
    assert!(matches!(
        &decoded[0].ops[2].payload,
        EditOp::Seq(ListOp::Ins {
            item: SeqItem::BlockAtom {
                leaf: AtomLeaf::Unknown(_),
                ..
            },
            ..
        })
    ));

    // ④ changesets_contain_unknown == true; encode_changesets/to_durable_item 거부
    assert!(editor_codec::changesets_contain_unknown(&decoded));
    let reencode_attempt = editor_codec::encode_changesets(
        editor_codec::ReencodableChangesets::from_local_ops(decoded.clone()),
    );
    assert!(matches!(
        reencode_attempt,
        Err(CodecError::Encode(
            editor_codec::EncodeInvariant::UnknownPayloadEncode
        ))
    ));

    // ② 투영에서 자식 char가 자리표시자 블록에 정상 부착(고아 0)이고 BlockAtom
    // 자리표시자는 부모(Root)의 direct leaf 1 슬롯
    let g = editor_crdt::OpGraph::from_changesets(decoded).unwrap();
    let pd = editor_model::project_document(&editor_model::split_logs(&g).unwrap()).unwrap();
    let root = pd.tree.root_node().unwrap();
    assert_eq!(
        root.children.len(),
        3,
        "placeholder block, placeholder leaf, real paragraph"
    );
    let editor_model::Child::Block(placeholder_id) = root.children.get(0).unwrap() else {
        panic!("expected placeholder block as root's first child");
    };
    let placeholder = pd.tree.get(*placeholder_id).unwrap();
    assert_eq!(placeholder.node_type, NodeType::Unknown);
    assert_eq!(placeholder.children.len(), 1);
    assert!(matches!(
        placeholder.children.get(0).unwrap(),
        editor_model::Child::Leaf {
            item: SeqItem::Char('x'),
            ..
        }
    ));
    assert!(matches!(
        root.children.get(1).unwrap(),
        editor_model::Child::Leaf {
            item: SeqItem::Atom(AtomLeaf::Unknown(_)),
            ..
        }
    ));
    let editor_model::Child::Block(para_id) = root.children.get(2).unwrap() else {
        panic!("expected the trailing real paragraph as root's third child");
    };
    let para = pd.tree.get(*para_id).unwrap();
    assert_eq!(para.node_type, NodeType::Paragraph);
    assert_eq!(para.children.len(), 1);

    // ③ durable 계층 재인코드 바이트 항등
    let bundle_css = decode_bundle(&bytes).unwrap();
    let envelope = editor_codec::envelope::unwrap(&bytes).unwrap();
    let mut b = &envelope.body[..];
    let dec = read_preamble(&mut b).unwrap();
    let reencoded = reencode_for_test(&bundle_css, &dec).unwrap();
    assert_eq!(reencoded, body_after_preamble(&envelope.body));

    // ④ (계속) ReencodableChangesets::from_local_ops 경유 to_durable_item도
    // EncodeInvariant 거부(panic 아님 — 값 수준 봉인), ⑤ into_reencodable()이 Fenced
    let decoded_again = editor_codec::decode_changesets(&bytes).unwrap();
    assert!(matches!(
        decoded_again.into_reencodable(),
        Err(CodecError::Fenced(Fenced::LossyForReencode))
    ));
}
