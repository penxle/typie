use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub trait DurableSchema {
    fn schema() -> TypeSchema;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeSchema {
    pub name: String,
    pub kind: SchemaKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaKind {
    EvolvableStruct {
        fields: Vec<FieldSchema>,
    },
    FrozenStruct {
        fields: Vec<FieldSchema>,
    },
    OpenEnum {
        variants: Vec<VariantSchema>,
        retired: Vec<u64>,
    },
    ClosedEnum {
        variants: Vec<VariantSchema>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefaultSchema {
    Required,
    Trait,
    Expr(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    pub ty: String,
    pub default: DefaultSchema,
}

impl FieldSchema {
    pub fn is_defaulted(&self) -> bool {
        !matches!(self.default, DefaultSchema::Required)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantSchema {
    pub name: String,
    pub tag: u64,
    pub frozen_payload: bool,
    pub fields: Vec<FieldSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeLock {
    pub magic: u8,
    pub format_version: u8,
    pub max_body_bytes: u64,
    pub compression_threshold_bytes: u64,
    pub known_flags: u8,
    pub payload_kinds: BTreeMap<String, u8>,
    pub required_features: BTreeMap<String, u64>,
    pub optional_features: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockDoc {
    pub envelope: EnvelopeLock,
    pub types: BTreeMap<String, TypeSchema>,
}

impl LockDoc {
    pub fn from_schemas(
        envelope: EnvelopeLock,
        schemas: impl IntoIterator<Item = TypeSchema>,
    ) -> Result<Self, String> {
        let mut types = BTreeMap::new();
        for schema in schemas {
            let name = schema.name.clone();
            if types.insert(name.clone(), schema).is_some() {
                return Err(format!("duplicate durable type name: {name}"));
            }
        }
        let doc = Self { envelope, types };
        doc.validate()?;
        Ok(doc)
    }

    /// 구조 무결성 검증 — 생성(from_schemas)과 역직렬화(parse) 양 경로가 공유한다.
    /// 변형된 락파일이 검증을 우회해 check_evolution의 맵 구축을 오염시키는 것을 차단.
    pub fn validate(&self) -> Result<(), String> {
        let mut kind_values = BTreeSet::new();
        for value in self.envelope.payload_kinds.values() {
            if !kind_values.insert(*value) {
                return Err(format!("duplicate payload kind value: {value}"));
            }
        }
        let mut feature_bits = BTreeSet::new();
        for value in self
            .envelope
            .required_features
            .values()
            .chain(self.envelope.optional_features.values())
        {
            if *value == 0 || (*value & (*value - 1)) != 0 {
                return Err(format!("feature bit must be a single nonzero bit: {value}"));
            }
            if !feature_bits.insert(*value) {
                return Err(format!("duplicate feature bit: {value}"));
            }
        }
        for (name, ty) in &self.types {
            let (variants, retired): (&[VariantSchema], &[u64]) = match &ty.kind {
                SchemaKind::OpenEnum { variants, retired } => {
                    (variants.as_slice(), retired.as_slice())
                }
                SchemaKind::ClosedEnum { variants } => (variants.as_slice(), &[][..]),
                _ => continue,
            };
            let mut tags = BTreeSet::new();
            for variant in variants {
                if !tags.insert(variant.tag) {
                    return Err(format!("{name}: duplicate variant tag {}", variant.tag));
                }
            }
            let mut retired_set = BTreeSet::new();
            for tag in retired {
                if !retired_set.insert(*tag) {
                    return Err(format!("{name}: duplicate retired tag {tag}"));
                }
                if tags.contains(tag) {
                    return Err(format!("{name}: tag {tag} is both active and retired"));
                }
            }
        }
        Ok(())
    }

    pub fn render(&self) -> String {
        serde_json::to_string_pretty(self).expect("LockDoc is always serializable")
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let doc: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        doc.validate()?;
        Ok(doc)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    TypeRemoved {
        ty: String,
    },
    KindChanged {
        ty: String,
    },
    FrozenChanged {
        ty: String,
    },
    FieldPrefixBroken {
        ty: String,
        context: String,
        index: usize,
    },
    AppendedFieldNotDefaulted {
        ty: String,
        context: String,
        field: String,
    },
    VariantRemovedWithoutRetire {
        ty: String,
        tag: u64,
    },
    RetiredTagReused {
        ty: String,
        tag: u64,
    },
    RetiredShrunk {
        ty: String,
        tag: u64,
    },
    VariantRenamed {
        ty: String,
        tag: u64,
    },
    VariantTagMoved {
        ty: String,
        name: String,
    },
    VariantPayloadGradeChanged {
        ty: String,
        tag: u64,
    },
    FrozenPayloadChanged {
        ty: String,
        tag: u64,
    },
    EnvelopeChanged {
        field: String,
    },
    FlagWithoutFeature {
        bits: u8,
    },
}

fn check_envelope(old: &EnvelopeLock, new: &EnvelopeLock, out: &mut Vec<Violation>) {
    if old.magic != new.magic {
        out.push(Violation::EnvelopeChanged {
            field: "magic".into(),
        });
    }
    if old.format_version != new.format_version {
        out.push(Violation::EnvelopeChanged {
            field: "format_version".into(),
        });
    }
    if old.max_body_bytes != new.max_body_bytes {
        out.push(Violation::EnvelopeChanged {
            field: "max_body_bytes".into(),
        });
    }
    if old.compression_threshold_bytes != new.compression_threshold_bytes {
        out.push(Violation::EnvelopeChanged {
            field: "compression_threshold_bytes".into(),
        });
    }
    if old.known_flags & !new.known_flags != 0 {
        out.push(Violation::EnvelopeChanged {
            field: "known_flags".into(),
        });
    }
    let new_flag_bits = new.known_flags & !old.known_flags;
    if new_flag_bits != 0 {
        let has_new_required = new
            .required_features
            .keys()
            .any(|k| !old.required_features.contains_key(k));
        if !has_new_required {
            out.push(Violation::FlagWithoutFeature {
                bits: new_flag_bits,
            });
        }
    }
    for (name, value) in &old.payload_kinds {
        if new.payload_kinds.get(name) != Some(value) {
            out.push(Violation::EnvelopeChanged {
                field: format!("payload_kinds.{name}"),
            });
        }
    }
    for (name, value) in &old.required_features {
        if new.required_features.get(name) != Some(value) {
            out.push(Violation::EnvelopeChanged {
                field: format!("required_features.{name}"),
            });
        }
    }
    for (name, value) in &old.optional_features {
        if new.optional_features.get(name) != Some(value) {
            out.push(Violation::EnvelopeChanged {
                field: format!("optional_features.{name}"),
            });
        }
    }
}

/// 신규 variant 추가는 구조적으로 additive이지만, 위치 산술/replay 의미론에 영향을 주는지의
/// 펜스 판정은 인간의 몫이다 — CI/릴리스 체크리스트가 이 목록을 리뷰 표면으로 쓴다.
pub fn added_variants(old: &LockDoc, new: &LockDoc) -> Vec<(String, String, u64)> {
    let mut out = Vec::new();
    for (name, new_ty) in &new.types {
        let (SchemaKind::OpenEnum {
            variants: new_variants,
            ..
        }
        | SchemaKind::ClosedEnum {
            variants: new_variants,
        }) = &new_ty.kind
        else {
            continue;
        };
        let old_tags: BTreeSet<u64> = match old.types.get(name).map(|t| &t.kind) {
            Some(SchemaKind::OpenEnum { variants, .. })
            | Some(SchemaKind::ClosedEnum { variants }) => variants.iter().map(|v| v.tag).collect(),
            _ => BTreeSet::new(),
        };
        for variant in new_variants {
            if !old_tags.contains(&variant.tag) {
                out.push((name.clone(), variant.name.clone(), variant.tag));
            }
        }
    }
    out
}

pub fn check_evolution(old: &LockDoc, new: &LockDoc) -> Vec<Violation> {
    let mut out = Vec::new();
    check_envelope(&old.envelope, &new.envelope, &mut out);
    for (name, old_ty) in &old.types {
        let Some(new_ty) = new.types.get(name) else {
            out.push(Violation::TypeRemoved { ty: name.clone() });
            continue;
        };
        match (&old_ty.kind, &new_ty.kind) {
            (
                SchemaKind::EvolvableStruct { fields: old_fields },
                SchemaKind::EvolvableStruct { fields: new_fields },
            ) => check_evolvable_fields(name, "", old_fields, new_fields, &mut out),
            (
                SchemaKind::FrozenStruct { fields: old_fields },
                SchemaKind::FrozenStruct { fields: new_fields },
            ) => {
                if old_fields != new_fields {
                    out.push(Violation::FrozenChanged { ty: name.clone() });
                }
            }
            (
                SchemaKind::OpenEnum {
                    variants: old_variants,
                    retired: old_retired,
                },
                SchemaKind::OpenEnum {
                    variants: new_variants,
                    retired: new_retired,
                },
            ) => check_open_enum(
                name,
                old_variants,
                old_retired,
                new_variants,
                new_retired,
                &mut out,
            ),
            (
                SchemaKind::ClosedEnum {
                    variants: old_variants,
                },
                SchemaKind::ClosedEnum {
                    variants: new_variants,
                },
            ) => {
                if old_variants != new_variants {
                    out.push(Violation::FrozenChanged { ty: name.clone() });
                }
            }
            _ => out.push(Violation::KindChanged { ty: name.clone() }),
        }
    }
    out
}

fn check_evolvable_fields(
    ty: &str,
    context: &str,
    old: &[FieldSchema],
    new: &[FieldSchema],
    out: &mut Vec<Violation>,
) {
    for (index, old_field) in old.iter().enumerate() {
        if new.get(index) != Some(old_field) {
            out.push(Violation::FieldPrefixBroken {
                ty: ty.to_owned(),
                context: context.to_owned(),
                index,
            });
            return;
        }
    }
    for new_field in &new[old.len()..] {
        if !new_field.is_defaulted() {
            out.push(Violation::AppendedFieldNotDefaulted {
                ty: ty.to_owned(),
                context: context.to_owned(),
                field: new_field.name.clone(),
            });
        }
    }
}

fn check_open_enum(
    ty: &str,
    old_variants: &[VariantSchema],
    old_retired: &[u64],
    new_variants: &[VariantSchema],
    new_retired: &[u64],
    out: &mut Vec<Violation>,
) {
    let new_by_tag: BTreeMap<u64, &VariantSchema> =
        new_variants.iter().map(|v| (v.tag, v)).collect();
    let new_by_name: BTreeMap<&str, u64> = new_variants
        .iter()
        .map(|v| (v.name.as_str(), v.tag))
        .collect();
    let new_retired_set: BTreeSet<u64> = new_retired.iter().copied().collect();

    for old_variant in old_variants {
        if let Some(new_tag) = new_by_name.get(old_variant.name.as_str())
            && *new_tag != old_variant.tag
        {
            out.push(Violation::VariantTagMoved {
                ty: ty.to_owned(),
                name: old_variant.name.clone(),
            });
        }
    }

    for tag in old_retired {
        if !new_retired_set.contains(tag) {
            out.push(Violation::RetiredShrunk {
                ty: ty.to_owned(),
                tag: *tag,
            });
        }
    }
    for variant in new_variants {
        if new_retired_set.contains(&variant.tag) {
            out.push(Violation::RetiredTagReused {
                ty: ty.to_owned(),
                tag: variant.tag,
            });
        }
    }
    for tag in old_retired {
        if new_by_tag.contains_key(tag) && !new_retired_set.contains(tag) {
            out.push(Violation::RetiredTagReused {
                ty: ty.to_owned(),
                tag: *tag,
            });
        }
    }

    for old_variant in old_variants {
        match new_by_tag.get(&old_variant.tag) {
            None => {
                if !new_retired_set.contains(&old_variant.tag) {
                    out.push(Violation::VariantRemovedWithoutRetire {
                        ty: ty.to_owned(),
                        tag: old_variant.tag,
                    });
                }
            }
            Some(new_variant) => {
                if new_variant.name != old_variant.name {
                    out.push(Violation::VariantRenamed {
                        ty: ty.to_owned(),
                        tag: old_variant.tag,
                    });
                    continue;
                }
                if new_variant.frozen_payload != old_variant.frozen_payload {
                    out.push(Violation::VariantPayloadGradeChanged {
                        ty: ty.to_owned(),
                        tag: old_variant.tag,
                    });
                    continue;
                }
                if old_variant.frozen_payload {
                    if new_variant.fields != old_variant.fields {
                        out.push(Violation::FrozenPayloadChanged {
                            ty: ty.to_owned(),
                            tag: old_variant.tag,
                        });
                    }
                } else {
                    check_evolvable_fields(
                        ty,
                        &old_variant.name,
                        &old_variant.fields,
                        &new_variant.fields,
                        out,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(name: &str, ty: &str) -> FieldSchema {
        FieldSchema {
            name: name.into(),
            ty: ty.into(),
            default: DefaultSchema::Required,
        }
    }

    fn defaulted(name: &str, ty: &str, default: DefaultSchema) -> FieldSchema {
        FieldSchema {
            name: name.into(),
            ty: ty.into(),
            default,
        }
    }

    fn variant(
        name: &str,
        tag: u64,
        frozen_payload: bool,
        fields: Vec<FieldSchema>,
    ) -> VariantSchema {
        VariantSchema {
            name: name.into(),
            tag,
            frozen_payload,
            fields,
        }
    }

    fn evolvable(name: &str, fields: Vec<FieldSchema>) -> TypeSchema {
        TypeSchema {
            name: name.into(),
            kind: SchemaKind::EvolvableStruct { fields },
        }
    }

    fn open(name: &str, variants: Vec<VariantSchema>, retired: Vec<u64>) -> TypeSchema {
        TypeSchema {
            name: name.into(),
            kind: SchemaKind::OpenEnum { variants, retired },
        }
    }

    fn envelope() -> EnvelopeLock {
        EnvelopeLock {
            magic: 0xC2,
            format_version: 1,
            max_body_bytes: 1 << 30,
            compression_threshold_bytes: 256,
            known_flags: 0b0000_0001,
            payload_kinds: BTreeMap::from([("changeset-bundle".to_owned(), 0u8)]),
            required_features: BTreeMap::new(),
            optional_features: BTreeMap::new(),
        }
    }

    fn doc(schemas: Vec<TypeSchema>) -> LockDoc {
        LockDoc::from_schemas(envelope(), schemas).unwrap()
    }

    #[test]
    fn lock_render_parse_round_trip() {
        let lock = doc(vec![
            evolvable("A", vec![req("x", "u64")]),
            open("B", vec![variant("V", 0, true, vec![])], vec![9]),
        ]);
        assert_eq!(LockDoc::parse(&lock.render()).unwrap(), lock);
    }

    #[test]
    fn duplicate_type_name_is_rejected() {
        let result = LockDoc::from_schemas(
            envelope(),
            vec![evolvable("A", vec![]), evolvable("A", vec![])],
        );
        assert!(result.is_err());
    }

    #[test]
    fn identical_docs_have_no_violations() {
        let lock = doc(vec![evolvable("A", vec![req("x", "u64")])]);
        assert_eq!(check_evolution(&lock, &lock), vec![]);
    }

    #[test]
    fn legal_evolutions_pass() {
        let old = doc(vec![
            evolvable("A", vec![req("x", "u64")]),
            open("E", vec![variant("V", 0, true, vec![])], vec![]),
        ]);
        let mut new_envelope = envelope();
        new_envelope.payload_kinds.insert("snapshot".to_owned(), 2);
        new_envelope
            .optional_features
            .insert("columnar".to_owned(), 0b10);
        let new = LockDoc::from_schemas(
            new_envelope,
            vec![
                evolvable(
                    "A",
                    vec![
                        req("x", "u64"),
                        defaulted("y", "Option<String>", DefaultSchema::Trait),
                    ],
                ),
                open(
                    "E",
                    vec![
                        variant("V", 0, true, vec![]),
                        variant("W", 1, false, vec![req("a", "u64")]),
                    ],
                    vec![],
                ),
                evolvable("New", vec![req("n", "u32")]),
            ],
        )
        .unwrap();
        assert_eq!(check_evolution(&old, &new), vec![]);
    }

    #[test]
    fn default_expr_change_breaks_prefix() {
        let old = doc(vec![evolvable(
            "A",
            vec![
                req("x", "u64"),
                defaulted("n", "u32", DefaultSchema::Expr("7".into())),
            ],
        )]);
        let new = doc(vec![evolvable(
            "A",
            vec![
                req("x", "u64"),
                defaulted("n", "u32", DefaultSchema::Expr("8".into())),
            ],
        )]);
        assert_eq!(
            check_evolution(&old, &new),
            vec![Violation::FieldPrefixBroken {
                ty: "A".into(),
                context: "".into(),
                index: 1
            }]
        );
    }

    #[test]
    fn variant_tag_move_is_rejected() {
        // retire + 동명 재추가로 태그를 옮기는 우회 차단
        let old = doc(vec![open("E", vec![variant("V", 0, true, vec![])], vec![])]);
        let moved = doc(vec![open(
            "E",
            vec![variant("V", 3, true, vec![])],
            vec![0],
        )]);
        assert!(
            check_evolution(&old, &moved).contains(&Violation::VariantTagMoved {
                ty: "E".into(),
                name: "V".into()
            })
        );
    }

    #[test]
    fn envelope_changes_are_rejected() {
        let old = doc(vec![]);
        let mut changed = envelope();
        changed.format_version = 2;
        let new = LockDoc::from_schemas(changed, vec![]).unwrap();
        assert_eq!(
            check_evolution(&old, &new),
            vec![Violation::EnvelopeChanged {
                field: "format_version".into()
            }]
        );

        let mut kind_moved = envelope();
        kind_moved
            .payload_kinds
            .insert("changeset-bundle".to_owned(), 7);
        let new = LockDoc::from_schemas(kind_moved, vec![]).unwrap();
        assert_eq!(
            check_evolution(&old, &new),
            vec![Violation::EnvelopeChanged {
                field: "payload_kinds.changeset-bundle".into()
            }]
        );
    }

    #[test]
    fn new_flag_requires_new_required_feature() {
        let old = doc(vec![]);
        let mut flag_only = envelope();
        flag_only.known_flags |= 0b10;
        let bad = LockDoc::from_schemas(flag_only.clone(), vec![]).unwrap();
        assert_eq!(
            check_evolution(&old, &bad),
            vec![Violation::FlagWithoutFeature { bits: 0b10 }]
        );

        let mut with_feature = flag_only;
        with_feature
            .required_features
            .insert("new-compression".to_owned(), 0b100);
        let ok = LockDoc::from_schemas(with_feature, vec![]).unwrap();
        assert_eq!(check_evolution(&old, &ok), vec![]);
    }

    #[test]
    fn envelope_registry_validation_rejects_duplicates() {
        let mut dup_kind = envelope();
        dup_kind.payload_kinds.insert("snapshot".to_owned(), 0);
        assert!(LockDoc::from_schemas(dup_kind, vec![]).is_err());

        let mut dup_bit = envelope();
        dup_bit.required_features.insert("a".to_owned(), 0b10);
        dup_bit.optional_features.insert("b".to_owned(), 0b10);
        assert!(LockDoc::from_schemas(dup_bit, vec![]).is_err());

        let mut multi_bit = envelope();
        multi_bit.required_features.insert("a".to_owned(), 0b110);
        assert!(LockDoc::from_schemas(multi_bit, vec![]).is_err());
    }

    #[test]
    fn frozen_payload_and_closed_enum_immutability() {
        let old = doc(vec![
            open(
                "E",
                vec![variant("V", 0, true, vec![req("c", "char")])],
                vec![],
            ),
            TypeSchema {
                name: "C".into(),
                kind: SchemaKind::ClosedEnum {
                    variants: vec![variant("A", 0, true, vec![])],
                },
            },
        ]);
        let frozen_payload_changed = doc(vec![
            open(
                "E",
                vec![variant("V", 0, true, vec![req("c", "u32")])],
                vec![],
            ),
            old.types["C"].clone(),
        ]);
        assert_eq!(
            check_evolution(&old, &frozen_payload_changed),
            vec![Violation::FrozenPayloadChanged {
                ty: "E".into(),
                tag: 0
            }]
        );

        let closed_changed = doc(vec![
            old.types["E"].clone(),
            TypeSchema {
                name: "C".into(),
                kind: SchemaKind::ClosedEnum {
                    variants: vec![variant("A", 0, true, vec![]), variant("B", 1, true, vec![])],
                },
            },
        ]);
        assert_eq!(
            check_evolution(&old, &closed_changed),
            vec![Violation::FrozenChanged { ty: "C".into() }]
        );
    }

    #[test]
    fn parse_rejects_tampered_lockfile() {
        // 직렬화 우회 변형(활성 태그가 retired에도 존재)은 parse의 validate에서 걸린다
        let mut bad = doc(vec![open(
            "E",
            vec![variant("V", 0, true, vec![])],
            vec![9],
        )]);
        let SchemaKind::OpenEnum { retired, .. } = &mut bad.types.get_mut("E").unwrap().kind else {
            panic!("E must be open");
        };
        retired.push(0);
        assert!(LockDoc::parse(&bad.render()).is_err());
    }

    #[test]
    fn added_variants_are_reported() {
        let old = doc(vec![open("E", vec![variant("V", 0, true, vec![])], vec![])]);
        let new = doc(vec![open(
            "E",
            vec![variant("V", 0, true, vec![]), variant("W", 1, true, vec![])],
            vec![],
        )]);
        assert_eq!(
            added_variants(&old, &new),
            vec![("E".to_owned(), "W".to_owned(), 1)]
        );
    }

    #[test]
    fn variant_retire_is_legal_but_removal_is_not() {
        let old = doc(vec![open("E", vec![variant("V", 0, true, vec![])], vec![])]);
        let retired = doc(vec![open("E", vec![], vec![0])]);
        assert_eq!(check_evolution(&old, &retired), vec![]);
        let removed = doc(vec![open("E", vec![], vec![])]);
        assert_eq!(
            check_evolution(&old, &removed),
            vec![Violation::VariantRemovedWithoutRetire {
                ty: "E".into(),
                tag: 0
            }]
        );
    }

    #[test]
    fn structural_violations_are_reported() {
        let old = doc(vec![
            evolvable("A", vec![req("x", "u64")]),
            TypeSchema {
                name: "F".into(),
                kind: SchemaKind::FrozenStruct {
                    fields: vec![req("x", "u64")],
                },
            },
        ]);
        let prefix_broken = doc(vec![
            evolvable("A", vec![req("x", "u32")]),
            old.types["F"].clone(),
        ]);
        assert_eq!(
            check_evolution(&old, &prefix_broken),
            vec![Violation::FieldPrefixBroken {
                ty: "A".into(),
                context: "".into(),
                index: 0
            }]
        );

        let appended_required = doc(vec![
            evolvable("A", vec![req("x", "u64"), req("y", "u64")]),
            old.types["F"].clone(),
        ]);
        assert_eq!(
            check_evolution(&old, &appended_required),
            vec![Violation::AppendedFieldNotDefaulted {
                ty: "A".into(),
                context: "".into(),
                field: "y".into()
            }]
        );

        let frozen_changed = doc(vec![
            old.types["A"].clone(),
            TypeSchema {
                name: "F".into(),
                kind: SchemaKind::FrozenStruct {
                    fields: vec![req("x", "u32")],
                },
            },
        ]);
        assert_eq!(
            check_evolution(&old, &frozen_changed),
            vec![Violation::FrozenChanged { ty: "F".into() }]
        );

        let kind_changed = doc(vec![
            TypeSchema {
                name: "A".into(),
                kind: SchemaKind::FrozenStruct { fields: vec![] },
            },
            old.types["F"].clone(),
        ]);
        assert_eq!(
            check_evolution(&old, &kind_changed),
            vec![Violation::KindChanged { ty: "A".into() }]
        );

        let removed = doc(vec![old.types["F"].clone()]);
        assert_eq!(
            check_evolution(&old, &removed),
            vec![Violation::TypeRemoved { ty: "A".into() }]
        );
    }

    #[test]
    fn tag_lifecycle_violations_are_reported() {
        let old = doc(vec![open(
            "E",
            vec![variant("V", 0, true, vec![])],
            vec![9],
        )]);

        let retired_reused = doc(vec![open(
            "E",
            vec![variant("V", 0, true, vec![]), variant("Z", 9, true, vec![])],
            vec![],
        )]);
        assert!(
            check_evolution(&old, &retired_reused).contains(&Violation::RetiredTagReused {
                ty: "E".into(),
                tag: 9
            })
        );

        let retired_shrunk = doc(vec![open("E", vec![variant("V", 0, true, vec![])], vec![])]);
        assert_eq!(
            check_evolution(&old, &retired_shrunk),
            vec![Violation::RetiredShrunk {
                ty: "E".into(),
                tag: 9
            }]
        );

        let renamed = doc(vec![open(
            "E",
            vec![variant("W", 0, true, vec![])],
            vec![9],
        )]);
        assert_eq!(
            check_evolution(&old, &renamed),
            vec![Violation::VariantRenamed {
                ty: "E".into(),
                tag: 0
            }]
        );

        let grade_changed = doc(vec![open(
            "E",
            vec![variant("V", 0, false, vec![req("a", "u64")])],
            vec![9],
        )]);
        assert_eq!(
            check_evolution(&old, &grade_changed),
            vec![Violation::VariantPayloadGradeChanged {
                ty: "E".into(),
                tag: 0
            }]
        );
    }

    #[test]
    fn evolvable_variant_payload_follows_field_rules() {
        let old = doc(vec![open(
            "E",
            vec![variant("V", 0, false, vec![req("a", "u64")])],
            vec![],
        )]);
        let appended_ok = doc(vec![open(
            "E",
            vec![variant(
                "V",
                0,
                false,
                vec![
                    req("a", "u64"),
                    defaulted("b", "Option<u32>", DefaultSchema::Trait),
                ],
            )],
            vec![],
        )]);
        assert_eq!(check_evolution(&old, &appended_ok), vec![]);

        let payload_prefix_broken = doc(vec![open(
            "E",
            vec![variant("V", 0, false, vec![req("a", "u32")])],
            vec![],
        )]);
        assert_eq!(
            check_evolution(&old, &payload_prefix_broken),
            vec![Violation::FieldPrefixBroken {
                ty: "E".into(),
                context: "V".into(),
                index: 0
            }]
        );
    }
}
