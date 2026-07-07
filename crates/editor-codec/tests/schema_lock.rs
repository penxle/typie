use std::collections::BTreeSet;

use editor_codec::envelope;
use editor_codec::registry::all_type_schemas;
use editor_codec::schema::{
    EnvelopeLock, LockDoc, SchemaKind, TypeSchema, added_variants, check_evolution,
};

fn envelope_lock() -> EnvelopeLock {
    EnvelopeLock {
        magic: envelope::MAGIC,
        format_version: envelope::FORMAT_VERSION,
        max_body_bytes: envelope::MAX_BODY_BYTES,
        compression_threshold_bytes: envelope::COMPRESSION_THRESHOLD_BYTES as u64,
        known_flags: envelope::KNOWN_FLAGS,
        payload_kinds: std::collections::BTreeMap::from([
            ("changeset-bundle".to_owned(), 0),
            ("dots".to_owned(), 1),
            ("snapshot".to_owned(), 2),
        ]),
        required_features: std::collections::BTreeMap::new(),
        optional_features: std::collections::BTreeMap::new(),
    }
}

fn current_lock() -> LockDoc {
    LockDoc::from_schemas(envelope_lock(), all_type_schemas()).expect("registry schemas are valid")
}

#[test]
fn lockfile_matches_registry() {
    let rendered = current_lock().render();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/codec.lock");
    if std::env::var("UPDATE_LOCK").is_ok() {
        std::fs::write(path, &rendered).unwrap();
    }
    let on_disk =
        std::fs::read_to_string(path).expect("codec.lock 체크인 필요 — UPDATE_LOCK=1로 생성");
    assert_eq!(
        on_disk, rendered,
        "스키마 변경은 codec.lock 재생성 커밋을 동반해야 한다"
    );
}

#[test]
fn evolution_from_latest_frozen_is_legal() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/codec-schema-history");
    if std::env::var("UPDATE_LOCK").is_ok() && !std::path::Path::new(dir).join("v1.lock").exists() {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            std::path::Path::new(dir).join("v1.lock"),
            current_lock().render(),
        )
        .unwrap();
    }
    let mut frozen: Vec<_> = std::fs::read_dir(dir)
        .expect("codec-schema-history 필요")
        .map(|e| e.unwrap().path())
        .collect();
    frozen.sort_by_key(|p| {
        p.file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_prefix('v'))
            .and_then(|n| n.parse::<u64>().ok())
            .expect("v{N}.lock 명명 규약")
    });
    let latest = frozen.last().expect("동결본 최소 1개");
    let old = LockDoc::parse(&std::fs::read_to_string(latest).unwrap()).unwrap();
    let new = current_lock();
    let violations = check_evolution(&old, &new);
    assert!(violations.is_empty(), "불법 진화: {violations:?}");
    for (ty, variant, tag) in added_variants(&old, &new) {
        eprintln!(
            "added variant since {}: {ty}::{variant} = {tag}",
            latest.display()
        );
    }
}

#[test]
fn every_derived_durable_type_is_registered() {
    let sources = [
        include_str!("../src/types/anchor.rs"),
        include_str!("../src/types/attr.rs"),
        include_str!("../src/types/item.rs"),
        include_str!("../src/types/modifier.rs"),
        include_str!("../src/types/op.rs"),
        include_str!("../src/types/values.rs"),
    ];
    let derived: usize = sources
        .iter()
        .map(|s| s.matches("Durable)]").count() + s.matches("Durable,").count())
        .sum();
    let registered = all_type_schemas().len();
    assert_eq!(
        derived, registered,
        "types/에 Durable 파생 {derived}개, 레지스트리 등록 {registered}개 — 새 타입을 registry.rs에 등록하라"
    );
    let mut names: Vec<String> = all_type_schemas().into_iter().map(|t| t.name).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), registered, "레지스트리 중복 등록");
}

fn collect_field_type_strings(ty: &TypeSchema) -> Vec<String> {
    match &ty.kind {
        SchemaKind::EvolvableStruct { fields } | SchemaKind::FrozenStruct { fields } => {
            fields.iter().map(|f| f.ty.clone()).collect()
        }
        SchemaKind::OpenEnum { variants, .. } | SchemaKind::ClosedEnum { variants } => variants
            .iter()
            .flat_map(|v| v.fields.iter().map(|f| f.ty.clone()))
            .collect(),
    }
}

fn attr_type_universe(schemas: &[TypeSchema]) -> BTreeSet<String> {
    let mut universe: BTreeSet<String> = BTreeSet::from(["DurableAttr".to_owned()]);
    loop {
        let mut grew = false;
        for ty in schemas {
            if universe.contains(&ty.name) {
                continue;
            }
            let reachable = universe.iter().any(|member| {
                schemas.iter().find(|t| &t.name == member).is_some_and(|t| {
                    collect_field_type_strings(t)
                        .iter()
                        .any(|field_ty| field_ty.contains(ty.name.as_str()))
                })
            });
            if reachable {
                universe.insert(ty.name.clone());
                grew = true;
            }
        }
        if !grew {
            return universe;
        }
    }
}

#[test]
fn attr_type_universe_is_dot_free() {
    let schemas = all_type_schemas();
    let universe = attr_type_universe(&schemas);
    for ty in &schemas {
        if !universe.contains(&ty.name) {
            continue;
        }
        for field_ty in collect_field_type_strings(ty) {
            assert!(
                !field_ty.contains("Dot"),
                "{}의 필드 타입 {}에 Dot — attr 우주는 ctx-독립이어야 한다",
                ty.name,
                field_ty
            );
        }
    }
}

#[test]
fn modifier_and_kind_share_one_tag_table() {
    let schemas = all_type_schemas();
    let table = |name: &str| -> Vec<(String, u64)> {
        let ty = schemas.iter().find(|t| t.name == name).expect(name);
        let SchemaKind::OpenEnum { variants, .. } = &ty.kind else {
            panic!("{name} must be an open enum");
        };
        variants.iter().map(|v| (v.name.clone(), v.tag)).collect()
    };
    assert_eq!(
        table("DurableModifier"),
        table("DurableModifierKind"),
        "modifier와 kind는 단일 태그 테이블을 공유해야 한다"
    );
}
