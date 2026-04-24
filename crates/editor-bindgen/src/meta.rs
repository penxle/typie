use bitcode::{Decode, Encode};

#[derive(Debug, Clone, Encode, Decode)]
pub struct FfiMeta {
    pub name: String,
    pub kind: FfiKind,
    pub serde_rename_all: Option<String>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum FfiKind {
    Struct {
        fields: Vec<FfiField>,
    },
    Enum {
        variants: Vec<FfiVariant>,
        serde_tag: Option<String>,
        default_variant: Option<String>,
    },
    Custom {
        target: String,
    },
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct FfiField {
    pub name: String,
    pub ty: String,
    pub has_serde_default: bool,
    pub ffi_default_override: Option<String>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum FfiVariant {
    Unit {
        name: String,
    },
    Tuple {
        name: String,
        tys: Vec<String>,
    },
    Struct {
        name: String,
        fields: Vec<FfiField>,
        serde_rename_all: Option<String>,
    },
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct FfiInterface {
    pub name: String,
    pub methods: Vec<FfiMethod>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct FfiMethod {
    pub name: String,
    pub is_async: bool,
    pub is_constructor: bool,
    pub params: Vec<FfiParam>,
    pub return_type: FfiReturnType,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct FfiParam {
    pub name: String,
    pub ty: FfiParamType,
}

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
pub enum FfiScalarParam {
    Primitive(String),
    Complex(String),
}

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
pub enum FfiParamType {
    Primitive(String),
    Complex(String),
    Vec(FfiScalarParam),
    Option(FfiScalarParam),
}

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
pub enum FfiScalarReturn {
    Primitive(String),
    Complex(String),
    Owned(String),
}

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
pub enum FfiReturnType {
    Unit,
    Primitive(String),
    Complex(String),
    Owned(String),
    Vec(FfiScalarReturn),
    Option(FfiScalarReturn),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_struct() {
        let meta = FfiMeta {
            name: "Position".into(),
            serde_rename_all: Some("snake_case".into()),
            kind: FfiKind::Struct {
                fields: vec![
                    FfiField {
                        name: "node_id".into(),
                        ty: "NodeId".into(),
                        has_serde_default: false,
                        ffi_default_override: None,
                    },
                    FfiField {
                        name: "offset".into(),
                        ty: "usize".into(),
                        has_serde_default: false,
                        ffi_default_override: None,
                    },
                    FfiField {
                        name: "affinity".into(),
                        ty: "Affinity".into(),
                        has_serde_default: true,
                        ffi_default_override: None,
                    },
                ],
            },
        };
        let encoded = bitcode::encode(&meta);
        let decoded: FfiMeta = bitcode::decode(&encoded).unwrap();
        assert_eq!(decoded.name, "Position");
        assert_eq!(decoded.serde_rename_all.as_deref(), Some("snake_case"));
        match decoded.kind {
            FfiKind::Struct { fields } => {
                assert_eq!(fields.len(), 3);
                assert_eq!(fields[0].name, "node_id");
                assert!(!fields[0].has_serde_default);
                assert!(fields[2].has_serde_default);
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn roundtrip_enum() {
        let meta = FfiMeta {
            name: "Affinity".into(),
            serde_rename_all: Some("snake_case".into()),
            kind: FfiKind::Enum {
                variants: vec![
                    FfiVariant::Unit {
                        name: "Downstream".into(),
                    },
                    FfiVariant::Unit {
                        name: "Upstream".into(),
                    },
                ],
                serde_tag: None,
                default_variant: Some("Downstream".into()),
            },
        };
        let encoded = bitcode::encode(&meta);
        let decoded: FfiMeta = bitcode::decode(&encoded).unwrap();
        assert_eq!(decoded.name, "Affinity");
        match decoded.kind {
            FfiKind::Enum {
                variants,
                serde_tag,
                default_variant,
            } => {
                assert_eq!(variants.len(), 2);
                assert!(serde_tag.is_none());
                assert_eq!(default_variant.as_deref(), Some("Downstream"));
            }
            _ => panic!("expected enum"),
        }
    }

    #[test]
    fn roundtrip_tagged_enum() {
        let meta = FfiMeta {
            name: "EditorEvent".into(),
            serde_rename_all: Some("snake_case".into()),
            kind: FfiKind::Enum {
                variants: vec![FfiVariant::Struct {
                    name: "StateChanged".into(),
                    fields: vec![FfiField {
                        name: "fields".into(),
                        ty: "Vec<StateField>".into(),
                        has_serde_default: false,
                        ffi_default_override: None,
                    }],
                    serde_rename_all: None,
                }],
                serde_tag: Some("type".into()),
                default_variant: None,
            },
        };
        let encoded = bitcode::encode(&meta);
        let decoded: FfiMeta = bitcode::decode(&encoded).unwrap();
        match decoded.kind {
            FfiKind::Enum { serde_tag, .. } => {
                assert_eq!(serde_tag.as_deref(), Some("type"));
            }
            _ => panic!("expected enum"),
        }
    }

    #[test]
    fn roundtrip_custom() {
        let meta = FfiMeta {
            name: "NodeId".into(),
            serde_rename_all: None,
            kind: FfiKind::Custom {
                target: "String".into(),
            },
        };
        let encoded = bitcode::encode(&meta);
        let decoded: FfiMeta = bitcode::decode(&encoded).unwrap();
        match decoded.kind {
            FfiKind::Custom { target } => assert_eq!(target, "String"),
            _ => panic!("expected custom"),
        }
    }

    #[test]
    fn roundtrip_ffi_default_override() {
        let meta = FfiMeta {
            name: "TableNode".into(),
            serde_rename_all: None,
            kind: FfiKind::Struct {
                fields: vec![FfiField {
                    name: "proportion".into(),
                    ty: "f32".into(),
                    has_serde_default: true,
                    ffi_default_override: Some("1.0f".into()),
                }],
            },
        };
        let encoded = bitcode::encode(&meta);
        let decoded: FfiMeta = bitcode::decode(&encoded).unwrap();
        match decoded.kind {
            FfiKind::Struct { fields } => {
                assert!(fields[0].has_serde_default);
                assert_eq!(fields[0].ffi_default_override.as_deref(), Some("1.0f"));
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn roundtrip_interface() {
        let iface = FfiInterface {
            name: "EditorHost".into(),
            methods: vec![
                FfiMethod {
                    name: "create_editor".into(),
                    is_async: false,
                    is_constructor: false,
                    params: vec![FfiParam {
                        name: "doc".into(),
                        ty: FfiParamType::Complex("Doc".into()),
                    }],
                    return_type: FfiReturnType::Owned("Editor".into()),
                },
                FfiMethod {
                    name: "load_icu_data".into(),
                    is_async: false,
                    is_constructor: false,
                    params: vec![FfiParam {
                        name: "data".into(),
                        ty: FfiParamType::Vec(FfiScalarParam::Primitive("u8".into())),
                    }],
                    return_type: FfiReturnType::Unit,
                },
            ],
        };
        let encoded = bitcode::encode(&iface);
        let decoded: FfiInterface = bitcode::decode(&encoded).unwrap();
        assert_eq!(decoded.name, "EditorHost");
        assert_eq!(decoded.methods.len(), 2);
        assert_eq!(
            decoded.methods[0].params[0].ty,
            FfiParamType::Complex("Doc".into())
        );
        assert_eq!(
            decoded.methods[0].return_type,
            FfiReturnType::Owned("Editor".into())
        );
        assert_eq!(
            decoded.methods[1].params[0].ty,
            FfiParamType::Vec(FfiScalarParam::Primitive("u8".into()))
        );
    }

    #[test]
    fn roundtrip_async_constructor() {
        let iface = FfiInterface {
            name: "EditorHost".into(),
            methods: vec![FfiMethod {
                name: "create".into(),
                is_async: true,
                is_constructor: true,
                params: vec![FfiParam {
                    name: "kind".into(),
                    ty: FfiParamType::Option(FfiScalarParam::Complex("BackendKind".into())),
                }],
                return_type: FfiReturnType::Owned("EditorHost".into()),
            }],
        };
        let encoded = bitcode::encode(&iface);
        let decoded: FfiInterface = bitcode::decode(&encoded).unwrap();
        assert!(decoded.methods[0].is_async);
        assert!(decoded.methods[0].is_constructor);
    }

    #[test]
    fn roundtrip_complex_return_types() {
        let iface = FfiInterface {
            name: "Editor".into(),
            methods: vec![
                FfiMethod {
                    name: "tick".into(),
                    is_async: false,
                    is_constructor: false,
                    params: vec![],
                    return_type: FfiReturnType::Vec(FfiScalarReturn::Complex("EditorEvent".into())),
                },
                FfiMethod {
                    name: "cursor".into(),
                    is_async: false,
                    is_constructor: false,
                    params: vec![],
                    return_type: FfiReturnType::Option(FfiScalarReturn::Complex(
                        "CursorMetrics".into(),
                    )),
                },
            ],
        };
        let encoded = bitcode::encode(&iface);
        let decoded: FfiInterface = bitcode::decode(&encoded).unwrap();
        assert_eq!(
            decoded.methods[0].return_type,
            FfiReturnType::Vec(FfiScalarReturn::Complex("EditorEvent".into()))
        );
    }
}
