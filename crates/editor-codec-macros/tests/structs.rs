use editor_codec::ctx::{CollectCtx, DecCtx, EncCtx};
use editor_codec::durable::Durable;
use editor_codec::framing::{UnknownTail, write_frame};
use editor_codec::varint::write_varint;
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

mod frozen {
    use editor_codec_macros::Durable;

    #[derive(Debug, PartialEq, Durable)]
    #[durable(frozen)]
    pub struct Anchor {
        pub id: u64,
        pub bias: u8,
    }
}

mod with_dot {
    use editor_codec::framing::UnknownTail;
    use editor_codec_macros::Durable;
    use editor_crdt::Dot;

    #[derive(Debug, PartialEq, Durable)]
    #[durable(evolvable)]
    pub struct NodeInit {
        pub id: Dot,
        pub label: String,
        pub tail: UnknownTail,
    }
}

mod v1 {
    use editor_codec::framing::UnknownTail;
    use editor_codec_macros::Durable;

    #[derive(Debug, PartialEq, Durable)]
    #[durable(evolvable)]
    pub struct Rec {
        pub a: u64,
        pub tail: UnknownTail,
    }
}

mod v2 {
    use editor_codec::framing::UnknownTail;
    use editor_codec_macros::Durable;

    #[derive(Debug, PartialEq, Durable)]
    #[durable(evolvable)]
    pub struct Rec {
        pub a: u64,
        #[durable(default)]
        pub b: Option<String>,
        pub tail: UnknownTail,
    }
}

mod required {
    use editor_codec::framing::UnknownTail;
    use editor_codec_macros::Durable;

    #[derive(Debug, PartialEq, Durable)]
    #[durable(evolvable)]
    pub struct Rec {
        pub a: u64,
        pub c: String,
        pub tail: UnknownTail,
    }
}

mod with_expr_default {
    use editor_codec::framing::UnknownTail;
    use editor_codec_macros::Durable;

    #[derive(Debug, PartialEq, Durable)]
    #[durable(evolvable)]
    pub struct Rec {
        pub a: u64,
        #[durable(default = "7")]
        pub n: u32,
        pub tail: UnknownTail,
    }
}

#[test]
fn frozen_struct_round_trips_frameless() {
    let a = frozen::Anchor { id: 300, bias: 1 };
    assert_eq!(round_trip(&a), a);

    let b = frozen::Anchor { id: 1, bias: 0 };
    let mut buf = Vec::new();
    a.encode(&empty_enc(), &mut buf).unwrap();
    b.encode(&empty_enc(), &mut buf).unwrap();
    let mut slice = &buf[..];
    assert_eq!(frozen::Anchor::decode(&empty_dec(), &mut slice).unwrap(), a);
    assert_eq!(frozen::Anchor::decode(&empty_dec(), &mut slice).unwrap(), b);
    assert!(slice.is_empty());
}

#[test]
fn evolvable_struct_with_dot_round_trips() {
    let rec = with_dot::NodeInit {
        id: Dot::new(7, 10),
        label: "블록".to_owned(),
        tail: UnknownTail::default(),
    };
    assert_eq!(round_trip(&rec), rec);
}

#[test]
fn old_reader_preserves_new_fields_byte_stable() {
    let new = v2::Rec {
        a: 42,
        b: Some("new".to_owned()),
        tail: UnknownTail::default(),
    };
    let mut original = Vec::new();
    new.encode(&empty_enc(), &mut original).unwrap();

    let mut slice = &original[..];
    let old = v1::Rec::decode(&empty_dec(), &mut slice).unwrap();
    assert!(slice.is_empty());
    assert_eq!(old.a, 42);
    assert!(!old.tail.0.is_empty());

    let mut reencoded = Vec::new();
    old.encode(&empty_enc(), &mut reencoded).unwrap();
    assert_eq!(reencoded, original);
}

#[test]
fn new_reader_defaults_missing_fields() {
    let old = v1::Rec {
        a: 7,
        tail: UnknownTail::default(),
    };
    let mut bytes = Vec::new();
    old.encode(&empty_enc(), &mut bytes).unwrap();
    let mut slice = &bytes[..];
    let new = v2::Rec::decode(&empty_dec(), &mut slice).unwrap();
    assert_eq!(
        new,
        v2::Rec {
            a: 7,
            b: None,
            tail: UnknownTail::default()
        }
    );
}

#[test]
fn missing_required_field_errors() {
    let mut bytes = Vec::new();
    write_frame(&mut bytes, |body| {
        write_varint(7, body);
        Ok(())
    })
    .unwrap();
    let mut slice = &bytes[..];
    assert!(matches!(
        required::Rec::decode(&empty_dec(), &mut slice),
        Err(CodecError::Corruption(Corruption::MissingRequiredField {
            ty: "Rec",
            field: "c"
        }))
    ));
}

#[test]
fn default_expr_applies_to_missing_field() {
    let mut bytes = Vec::new();
    write_frame(&mut bytes, |body| {
        write_varint(3, body);
        Ok(())
    })
    .unwrap();
    let mut slice = &bytes[..];
    let rec = with_expr_default::Rec::decode(&empty_dec(), &mut slice).unwrap();
    assert_eq!(
        rec,
        with_expr_default::Rec {
            a: 3,
            n: 7,
            tail: UnknownTail::default()
        }
    );
}
