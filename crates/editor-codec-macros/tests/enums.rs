use editor_codec::ctx::{CollectCtx, DecCtx, EncCtx};
use editor_codec::durable::Durable;
use editor_codec::framing::{UnknownPayload, UnknownTail, write_open_variant};
use editor_codec::primitives::write_char;
use editor_codec::{CodecError, Corruption};
use editor_crdt::Dot;

fn round_trip<T: Durable + PartialEq + std::fmt::Debug>(value: &T) -> T {
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

fn empty_enc() -> EncCtx {
    EncCtx::from_parts(&[], vec![]).unwrap()
}

fn empty_dec() -> DecCtx {
    DecCtx {
        actors: vec![],
        baselines: vec![],
    }
}

mod v1 {
    use editor_codec::framing::{UnknownPayload, UnknownTail};
    use editor_codec_macros::Durable;
    use editor_crdt::Dot;

    #[derive(Debug, PartialEq, Durable)]
    #[durable(open)]
    #[durable(retired(9))]
    pub enum Item {
        #[durable(n(0))]
        #[durable(frozen)]
        Char(char),
        #[durable(n(1))]
        Block {
            node_type: u32,
            parents: Vec<Dot>,
            tail: UnknownTail,
        },
        #[durable(n(2))]
        Break,
        #[durable(unknown)]
        Unknown(UnknownPayload),
    }
}

mod v2 {
    use editor_codec::framing::{UnknownPayload, UnknownTail};
    use editor_codec_macros::Durable;
    use editor_crdt::Dot;

    #[derive(Debug, PartialEq, Durable)]
    #[durable(open)]
    #[durable(retired(9))]
    pub enum Item {
        #[durable(n(0))]
        #[durable(frozen)]
        Char(char),
        #[durable(n(1))]
        Block {
            node_type: u32,
            parents: Vec<Dot>,
            tail: UnknownTail,
        },
        #[durable(n(2))]
        Break,
        #[durable(n(3))]
        Marker { weight: u16, tail: UnknownTail },
        #[durable(unknown)]
        Unknown(UnknownPayload),
    }
}

mod closed {
    use editor_codec_macros::Durable;

    #[derive(Debug, PartialEq, Durable)]
    #[durable(closed)]
    pub enum Bias {
        #[durable(n(0))]
        Before,
        #[durable(n(1))]
        After,
    }
}

#[test]
fn open_variants_round_trip() {
    assert_eq!(round_trip(&v1::Item::Char('한')), v1::Item::Char('한'));
    assert_eq!(round_trip(&v1::Item::Break), v1::Item::Break);
    let block = v1::Item::Block {
        node_type: 3,
        parents: vec![Dot::new(7, 10), Dot::new(99, 3)],
        tail: UnknownTail::default(),
    };
    assert_eq!(round_trip(&block), block);
}

#[test]
fn unknown_variant_preserved_byte_stable() {
    let mut original = Vec::new();
    write_open_variant(999, &mut original, |body| {
        body.extend_from_slice(&[0xde, 0xad]);
        Ok(())
    })
    .unwrap();

    let mut slice = &original[..];
    let decoded = v1::Item::decode(&empty_dec(), &mut slice).unwrap();
    assert_eq!(
        decoded,
        v1::Item::Unknown(UnknownPayload {
            tag: 999,
            bytes: vec![0xde, 0xad]
        })
    );

    let mut reencoded = Vec::new();
    decoded.encode(&empty_enc(), &mut reencoded).unwrap();
    assert_eq!(reencoded, original);
}

#[test]
fn vnext_variant_survives_old_reader_round_trip() {
    let new = v2::Item::Marker {
        weight: 500,
        tail: UnknownTail::default(),
    };
    let mut original = Vec::new();
    new.encode(&empty_enc(), &mut original).unwrap();

    let mut slice = &original[..];
    let old = v1::Item::decode(&empty_dec(), &mut slice).unwrap();
    assert!(matches!(&old, v1::Item::Unknown(u) if u.tag == 3));

    let mut relayed = Vec::new();
    old.encode(&empty_enc(), &mut relayed).unwrap();
    assert_eq!(relayed, original);

    let mut slice = &relayed[..];
    let recovered = v2::Item::decode(&empty_dec(), &mut slice).unwrap();
    assert_eq!(recovered, new);
}

#[test]
fn frozen_payload_rejects_trailing_bytes() {
    let mut bytes = Vec::new();
    write_open_variant(0, &mut bytes, |body| {
        write_char('x', body);
        body.push(0xff);
        Ok(())
    })
    .unwrap();
    let mut slice = &bytes[..];
    assert!(matches!(
        v1::Item::decode(&empty_dec(), &mut slice),
        Err(CodecError::Corruption(Corruption::TrailingBytes {
            remaining: 1
        }))
    ));
}

#[test]
fn unit_variant_rejects_payload_bytes() {
    let mut bytes = Vec::new();
    write_open_variant(2, &mut bytes, |body| {
        body.push(0xaa);
        Ok(())
    })
    .unwrap();
    let mut slice = &bytes[..];
    assert!(matches!(
        v1::Item::decode(&empty_dec(), &mut slice),
        Err(CodecError::Corruption(Corruption::TrailingBytes {
            remaining: 1
        }))
    ));
}

#[test]
fn closed_enum_round_trips() {
    assert_eq!(round_trip(&closed::Bias::Before), closed::Bias::Before);
    assert_eq!(round_trip(&closed::Bias::After), closed::Bias::After);
}

#[test]
fn closed_enum_unknown_tag_is_corruption() {
    let mut bytes = Vec::new();
    editor_codec::framing::write_closed_tag(5, &mut bytes);
    let mut slice = &bytes[..];
    assert!(matches!(
        closed::Bias::decode(&empty_dec(), &mut slice),
        Err(CodecError::Corruption(Corruption::UnknownClosedTag {
            ty: "Bias",
            tag: 5
        }))
    ));
}

mod unit_only {
    use editor_codec::framing::UnknownPayload;
    use editor_codec_macros::Durable;

    #[derive(Debug, PartialEq, Durable)]
    #[durable(open)]
    pub enum Marker {
        #[durable(n(0))]
        A,
        #[durable(n(1))]
        B,
        #[durable(unknown)]
        Unknown(UnknownPayload),
    }
}

#[test]
fn all_unit_open_enum_round_trips_and_preserves_unknown() {
    assert_eq!(round_trip(&unit_only::Marker::A), unit_only::Marker::A);
    assert_eq!(round_trip(&unit_only::Marker::B), unit_only::Marker::B);

    let mut original = Vec::new();
    write_open_variant(7, &mut original, |body| {
        body.push(0x01);
        Ok(())
    })
    .unwrap();
    let mut slice = &original[..];
    let decoded = unit_only::Marker::decode(&empty_dec(), &mut slice).unwrap();
    let mut reencoded = Vec::new();
    decoded.encode(&empty_enc(), &mut reencoded).unwrap();
    assert_eq!(reencoded, original);
}
