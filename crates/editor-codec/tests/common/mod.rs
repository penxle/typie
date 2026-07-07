use editor_codec::ctx::{EncCtx, write_dot, write_preamble};
use editor_codec::durable::Durable;
use editor_codec::envelope::{Envelope, FORMAT_VERSION, PayloadKind, wrap};
use editor_codec::framing::{UnknownPayload, write_frame};
use editor_codec::types::item::DurableItem;
use editor_codec::types::op::DurableOp;
use editor_codec::varint::write_varint;
use editor_crdt::Dot;

pub fn synth_unknown_bundle() -> Vec<u8> {
    let actors = [7u64];
    let baselines = [0u64];
    let ctx = EncCtx::from_parts(&actors, baselines.to_vec()).unwrap();
    let mut body = Vec::new();
    write_preamble(&actors, &baselines, &mut body).unwrap();
    write_varint(1, &mut body);
    body.push(0);
    write_varint(1, &mut body);
    write_frame(&mut body, |b| {
        write_dot(&Dot::new(7, 0), &ctx, b)?;
        DurableOp::Unknown(UnknownPayload {
            tag: 12345,
            bytes: vec![0xAB],
        })
        .encode(&ctx, b)
    })
    .unwrap();
    wrap(&Envelope::new(PayloadKind::ChangesetBundle, body)).unwrap()
}

pub fn synth_record_tail_bundle() -> Vec<u8> {
    let actors = [7u64];
    let baselines = [0u64];
    let ctx = EncCtx::from_parts(&actors, baselines.to_vec()).unwrap();
    let mut body = Vec::new();
    write_preamble(&actors, &baselines, &mut body).unwrap();
    write_varint(1, &mut body);
    body.push(0);
    write_varint(1, &mut body);
    write_frame(&mut body, |b| {
        write_dot(&Dot::new(7, 0), &ctx, b)?;
        DurableOp::SeqIns {
            pos: 0,
            item: DurableItem::Char('z'),
        }
        .encode(&ctx, b)?;
        b.extend_from_slice(&[0xEE, 0x07]);
        Ok(())
    })
    .unwrap();
    wrap(&Envelope::new(PayloadKind::ChangesetBundle, body)).unwrap()
}

pub fn synth_fenced_envelope() -> Vec<u8> {
    let mut bytes = wrap(&Envelope::new(PayloadKind::ChangesetBundle, b"x".to_vec())).unwrap();
    bytes[1] = FORMAT_VERSION + 1;
    bytes
}
