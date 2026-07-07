use std::collections::BTreeMap;

use editor_codec::envelope::{
    COMPRESSION_THRESHOLD_BYTES, FORMAT_VERSION, KNOWN_FLAGS, MAGIC, MAX_BODY_BYTES, PayloadKind,
};
use editor_codec::schema::{
    DefaultSchema, DurableSchema, EnvelopeLock, LockDoc, SchemaKind, check_evolution,
};

fn envelope_lock() -> EnvelopeLock {
    EnvelopeLock {
        magic: MAGIC,
        format_version: FORMAT_VERSION,
        max_body_bytes: MAX_BODY_BYTES,
        compression_threshold_bytes: COMPRESSION_THRESHOLD_BYTES as u64,
        known_flags: KNOWN_FLAGS,
        payload_kinds: BTreeMap::from([
            (
                "changeset-bundle".to_owned(),
                PayloadKind::ChangesetBundle as u8,
            ),
            ("dots".to_owned(), PayloadKind::Dots as u8),
            ("snapshot".to_owned(), PayloadKind::Snapshot as u8),
        ]),
        required_features: BTreeMap::new(),
        optional_features: BTreeMap::new(),
    }
}

mod v1 {
    use editor_codec::framing::{UnknownPayload, UnknownTail};
    use editor_codec_macros::Durable;

    #[derive(Durable)]
    #[durable(frozen)]
    pub struct Anchor {
        pub id: u64,
        pub bias: u8,
    }

    #[derive(Durable)]
    #[durable(evolvable)]
    pub struct Rec {
        pub a: u64,
        pub tail: UnknownTail,
    }

    #[derive(Durable)]
    #[durable(open)]
    #[durable(retired(9))]
    pub enum Item {
        #[durable(n(0))]
        #[durable(frozen)]
        Char(char),
        #[durable(n(2))]
        Break,
        #[durable(unknown)]
        Unknown(UnknownPayload),
    }

    #[derive(Durable)]
    #[durable(closed)]
    pub enum Bias {
        #[durable(n(0))]
        Before,
        #[durable(n(1))]
        After,
    }
}

mod v2 {
    use editor_codec::framing::{UnknownPayload, UnknownTail};
    use editor_codec_macros::Durable;

    #[derive(Durable)]
    #[durable(evolvable)]
    pub struct Rec {
        pub a: u64,
        #[durable(default)]
        pub b: Option<String>,
        pub tail: UnknownTail,
    }

    #[derive(Durable)]
    #[durable(open)]
    #[durable(retired(9))]
    pub enum Item {
        #[durable(n(0))]
        #[durable(frozen)]
        Char(char),
        #[durable(n(2))]
        Break,
        #[durable(n(3))]
        Marker { weight: u16, tail: UnknownTail },
        #[durable(unknown)]
        Unknown(UnknownPayload),
    }
}

fn v1_doc() -> LockDoc {
    LockDoc::from_schemas(
        envelope_lock(),
        vec![
            v1::Anchor::schema(),
            v1::Rec::schema(),
            v1::Item::schema(),
            v1::Bias::schema(),
        ],
    )
    .unwrap()
}

fn v2_doc() -> LockDoc {
    LockDoc::from_schemas(
        envelope_lock(),
        vec![
            v1::Anchor::schema(),
            v2::Rec::schema(),
            v2::Item::schema(),
            v1::Bias::schema(),
        ],
    )
    .unwrap()
}

#[test]
fn emitted_schemas_have_expected_shape() {
    let rec = v2::Rec::schema();
    let SchemaKind::EvolvableStruct { fields } = &rec.kind else {
        panic!("Rec must be evolvable: {rec:?}");
    };
    assert_eq!(fields.len(), 2, "tail은 스키마 필드에 포함되지 않는다");
    assert_eq!(
        (fields[0].name.as_str(), fields[0].ty.as_str()),
        ("a", "u64")
    );
    assert_eq!(fields[0].default, DefaultSchema::Required);
    assert_eq!(
        (fields[1].name.as_str(), fields[1].ty.as_str()),
        ("b", "Option<String>")
    );
    assert_eq!(fields[1].default, DefaultSchema::Trait);

    let item = v1::Item::schema();
    let SchemaKind::OpenEnum { variants, retired } = &item.kind else {
        panic!("Item must be open: {item:?}");
    };
    assert_eq!(retired, &vec![9]);
    assert_eq!(
        variants
            .iter()
            .map(|v| (v.name.as_str(), v.tag, v.frozen_payload))
            .collect::<Vec<_>>(),
        vec![("Char", 0, true), ("Break", 2, true)],
        "unknown variant는 스키마에 포함되지 않는다"
    );
}

#[test]
fn lock_fixture_regen_compare() {
    let rendered = v1_doc().render();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/synthetic.lock");
    if std::env::var("UPDATE_LOCK").is_ok() {
        std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures")).unwrap();
        std::fs::write(path, &rendered).unwrap();
    }
    let committed = std::fs::read_to_string(path).expect(
        "fixture missing — run once with UPDATE_LOCK=1 and commit tests/fixtures/synthetic.lock",
    );
    assert_eq!(
        rendered, committed,
        "락파일 재생성 불일치 — 스키마 변경이 락파일 diff 없이 통과하려 했음"
    );
}

#[test]
fn version_evolution_passes_checker() {
    assert_eq!(check_evolution(&v1_doc(), &v2_doc()), vec![]);
}

#[test]
fn hand_mutated_schema_fails_checker() {
    let old = v1_doc();
    let mut bad = v2_doc();
    let SchemaKind::EvolvableStruct { fields } = &mut bad.types.get_mut("Rec").unwrap().kind else {
        panic!("Rec must be evolvable");
    };
    fields[0].ty = "u32".into();
    assert!(!check_evolution(&old, &bad).is_empty());
}
