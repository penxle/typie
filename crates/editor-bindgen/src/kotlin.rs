use std::collections::{HashMap, HashSet};
use std::path::Path;

use heck::{
    ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase,
    ToUpperCamelCase,
};

use crate::meta::{FfiField, FfiKind, FfiMeta, FfiVariant};

const PACKAGE: &str = "co.typie.editor.ffi";

struct CodegenContext<'a> {
    custom_types: HashMap<String, String>,
    meta_map: HashMap<&'a str, &'a FfiMeta>,
    known_types: HashSet<&'a str>,
    inlined_types: HashSet<String>,
}

pub fn generate_all(metas: &[FfiMeta], output_dir: &Path) {
    let custom_types = build_custom_type_map(metas);

    let mut meta_map: HashMap<&str, &FfiMeta> = HashMap::new();
    for m in metas {
        meta_map.insert(&m.name, m);
    }

    let mut inlined_types = HashSet::new();
    for meta in metas {
        if let FfiKind::Enum {
            variants,
            serde_tag: Some(_),
            ..
        } = &meta.kind
        {
            for variant in variants {
                if let FfiVariant::Tuple { tys, .. } = variant {
                    if tys.len() == 1 {
                        let arg_type = &tys[0];
                        if let Some(arg_meta) = meta_map.get(arg_type.as_str()) {
                            if matches!(arg_meta.kind, FfiKind::Struct { .. }) {
                                inlined_types.insert(arg_type.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    let known_types: HashSet<&str> = meta_map.keys().copied().collect();

    let ctx = CodegenContext {
        custom_types,
        meta_map,
        known_types,
        inlined_types,
    };

    let pkg_dir = output_dir.join(PACKAGE.replace('.', "/"));
    std::fs::create_dir_all(&pkg_dir).expect("failed to create output directory");

    for meta in metas {
        if matches!(meta.kind, FfiKind::Custom { .. }) {
            continue;
        }
        if ctx.inlined_types.contains(&meta.name) {
            continue;
        }

        let content = match &meta.kind {
            FfiKind::Struct { fields } => generate_data_class(meta, fields, &ctx),
            FfiKind::Enum {
                variants,
                serde_tag,
                ..
            } => {
                if serde_tag.is_none()
                    && variants
                        .iter()
                        .all(|v| matches!(v, FfiVariant::Unit { .. }))
                {
                    generate_enum_class(meta, variants, &ctx)
                } else {
                    generate_sealed_class(meta, variants, &ctx)
                }
            }
            FfiKind::Custom { .. } => unreachable!(),
        };

        let path = pkg_dir.join(format!("{}.kt", meta.name));
        std::fs::write(&path, content).expect("failed to write file");
    }
}

fn build_custom_type_map(metas: &[FfiMeta]) -> HashMap<String, String> {
    metas
        .iter()
        .filter_map(|m| match &m.kind {
            FfiKind::Custom { target } => Some((m.name.clone(), target.clone())),
            _ => None,
        })
        .collect()
}

fn apply_rename(name: &str, strategy: Option<&str>) -> String {
    match strategy {
        Some("snake_case") => name.to_snake_case(),
        Some("camelCase") => name.to_lower_camel_case(),
        Some("PascalCase") => name.to_upper_camel_case(),
        Some("SCREAMING_SNAKE_CASE") => name.to_shouty_snake_case(),
        Some("kebab-case") => name.to_kebab_case(),
        Some("SCREAMING-KEBAB-CASE") => name.to_shouty_kebab_case(),
        _ => name.to_string(),
    }
}

fn resolve_default(field: &FfiField, kt_type: &str, ctx: &CodegenContext) -> String {
    if let Some(override_val) = &field.ffi_default_override {
        return override_val.clone();
    }

    let rust_ty = &field.ty;

    if rust_ty.starts_with("Option<") {
        return "null".into();
    }

    if rust_ty.starts_with("Vec<") || rust_ty.starts_with("imbl::Vector<") {
        return "emptyList()".into();
    }

    if let Some(meta) = ctx.meta_map.get(rust_ty.as_str()) {
        match &meta.kind {
            FfiKind::Enum {
                default_variant: Some(variant),
                ..
            } => {
                return format!("{PACKAGE}.{}.{}", meta.name, variant);
            }
            FfiKind::Struct { fields } if fields.iter().all(|f| f.has_serde_default) => {
                return format!("{PACKAGE}.{}()", meta.name);
            }
            _ => {}
        }
    }

    match rust_ty.as_str() {
        "bool" => "false".into(),
        "u8" | "u16" | "u32" | "i8" | "i16" | "i32" => "0".into(),
        "u64" | "i64" | "usize" => "0L".into(),
        "f32" => "0.0f".into(),
        "f64" => "0.0".into(),
        "String" => "\"\"".into(),
        _ => format!("{}()", kt_type),
    }
}

fn map_type(
    rust_ty: &str,
    custom_types: &HashMap<String, String>,
    known_types: &HashSet<&str>,
) -> String {
    let parsed: syn::Type = syn::parse_str(rust_ty).expect("failed to parse type");
    map_syn_type(&parsed, custom_types, known_types)
}

fn map_syn_type(
    ty: &syn::Type,
    custom_types: &HashMap<String, String>,
    known_types: &HashSet<&str>,
) -> String {
    match ty {
        syn::Type::Path(type_path) => map_type_path(type_path, custom_types, known_types),
        _ => panic!("unsupported type in FFI metadata"),
    }
}

fn map_type_path(
    type_path: &syn::TypePath,
    custom_types: &HashMap<String, String>,
    known_types: &HashSet<&str>,
) -> String {
    let path = &type_path.path;
    let segments: Vec<_> = path.segments.iter().collect();
    let last = segments.last().expect("empty path");
    let ident = last.ident.to_string();

    if segments.len() == 1 && last.arguments.is_none() {
        if let Some(target) = custom_types.get(&ident) {
            return map_type(target, custom_types, known_types);
        }
    }

    if segments.len() == 1 && last.arguments.is_none() {
        match ident.as_str() {
            "bool" => return "Boolean".into(),
            "u8" | "u16" | "u32" | "i8" | "i16" | "i32" => return "Int".into(),
            "u64" | "i64" | "usize" => return "Long".into(),
            "f32" => return "Float".into(),
            "f64" => return "Double".into(),
            "String" => return "String".into(),
            _ => {}
        }
    }

    let args = match &last.arguments {
        syn::PathArguments::AngleBracketed(args) => args,
        syn::PathArguments::None => {
            if known_types.contains(ident.as_str()) {
                return format!("{PACKAGE}.{ident}");
            }
            return ident;
        }
        _ => panic!("unsupported path arguments"),
    };

    let full_path: String = segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");

    match full_path.as_str() {
        "Option" => {
            let inner = extract_single_type_arg(args);
            format!("{}?", map_syn_type(inner, custom_types, known_types))
        }
        "Vec" | "imbl::Vector" => {
            let inner = extract_single_type_arg(args);
            format!("List<{}>", map_syn_type(inner, custom_types, known_types))
        }
        "HashMap" | "imbl::HashMap" | "std::collections::HashMap" | "hashbrown::HashMap" => {
            let mut arg_iter = args.args.iter();
            let key_ty = match arg_iter.next().expect("missing key type arg") {
                syn::GenericArgument::Type(ty) => ty,
                _ => panic!("expected type argument"),
            };
            let val_ty = match arg_iter.next().expect("missing value type arg") {
                syn::GenericArgument::Type(ty) => ty,
                _ => panic!("expected type argument"),
            };
            format!(
                "Map<{}, {}>",
                map_syn_type(key_ty, custom_types, known_types),
                map_syn_type(val_ty, custom_types, known_types)
            )
        }
        _ => ident,
    }
}

fn extract_single_type_arg(args: &syn::AngleBracketedGenericArguments) -> &syn::Type {
    match args.args.first().expect("empty generic args") {
        syn::GenericArgument::Type(ty) => ty,
        _ => panic!("expected type argument"),
    }
}

struct CodeWriter {
    buf: String,
    indent: usize,
}

impl CodeWriter {
    fn new() -> Self {
        Self {
            buf: String::new(),
            indent: 0,
        }
    }

    fn line(&mut self, s: &str) {
        if s.is_empty() {
            self.buf.push('\n');
        } else {
            for _ in 0..self.indent {
                self.buf.push_str("    ");
            }
            self.buf.push_str(s);
            self.buf.push('\n');
        }
    }

    fn open_block(&mut self, header: &str) {
        self.line(&format!("{} {{", header));
        self.indent += 1;
    }

    fn close_block(&mut self) {
        self.indent -= 1;
        self.line("}");
    }

    fn finish(self) -> String {
        self.buf
    }
}

fn generate_data_class(meta: &FfiMeta, fields: &[FfiField], ctx: &CodegenContext) -> String {
    let mut w = CodeWriter::new();
    w.line(&format!("package {}", PACKAGE));
    w.line("");
    w.line("import kotlinx.serialization.SerialName");
    w.line("import kotlinx.serialization.Serializable");
    w.line("");
    w.line("@Serializable");
    if fields.is_empty() {
        w.line(&format!("class {}", meta.name));
    } else {
        w.line(&format!("data class {}(", meta.name));
        w.indent += 1;
        for field in fields {
            let kt_name = field.name.to_lower_camel_case();
            let kt_type = map_type(&field.ty, &ctx.custom_types, &ctx.known_types);
            let serial_name = apply_rename(&field.name, meta.serde_rename_all.as_deref());
            let default_part = if field.has_serde_default {
                format!(" = {}", resolve_default(field, &kt_type, ctx))
            } else {
                String::new()
            };
            w.line(&format!(
                "@SerialName(\"{}\") val {}: {}{},",
                serial_name, kt_name, kt_type, default_part
            ));
        }
        w.indent -= 1;
        w.line(")");
    }
    w.finish()
}

fn generate_enum_class(meta: &FfiMeta, variants: &[FfiVariant], _ctx: &CodegenContext) -> String {
    let mut w = CodeWriter::new();
    w.line(&format!("package {}", PACKAGE));
    w.line("");
    w.line("import kotlinx.serialization.SerialName");
    w.line("import kotlinx.serialization.Serializable");
    w.line("");
    w.line("@Serializable");
    w.open_block(&format!("enum class {}", meta.name));
    for variant in variants {
        if let FfiVariant::Unit { name } = variant {
            let serial_name = apply_rename(name, meta.serde_rename_all.as_deref());
            w.line(&format!("@SerialName(\"{}\") {},", serial_name, name));
        }
    }
    w.close_block();
    w.finish()
}

fn generate_sealed_class(meta: &FfiMeta, variants: &[FfiVariant], ctx: &CodegenContext) -> String {
    let mut w = CodeWriter::new();
    w.line(&format!("package {}", PACKAGE));
    w.line("");
    w.line("import kotlinx.serialization.SerialName");
    w.line("import kotlinx.serialization.Serializable");
    w.line("");
    w.line("@Serializable");
    w.open_block(&format!("sealed class {}", meta.name));
    for variant in variants {
        match variant {
            FfiVariant::Unit { name } => {
                let serial_name = apply_rename(name, meta.serde_rename_all.as_deref());
                w.line("");
                w.line(&format!("@Serializable @SerialName(\"{}\")", serial_name));
                w.line(&format!("data object {} : {}()", name, meta.name));
            }
            FfiVariant::Tuple { name, tys } => {
                let serial_name = apply_rename(name, meta.serde_rename_all.as_deref());
                w.line("");
                w.line(&format!("@Serializable @SerialName(\"{}\")", serial_name));

                if tys.len() == 1 {
                    if let Some(inner_meta) = ctx.meta_map.get(tys[0].as_str()) {
                        if let FfiKind::Struct { fields } = &inner_meta.kind {
                            let rename_all = inner_meta.serde_rename_all.as_deref();
                            if fields.is_empty() {
                                w.line(&format!("data object {} : {}()", name, meta.name));
                            } else {
                                let params = fields
                                    .iter()
                                    .map(|f| {
                                        let kt_name = f.name.to_lower_camel_case();
                                        let kt_type =
                                            map_type(&f.ty, &ctx.custom_types, &ctx.known_types);
                                        let sn = apply_rename(&f.name, rename_all);
                                        let default_part = if f.has_serde_default {
                                            format!(" = {}", resolve_default(f, &kt_type, ctx))
                                        } else {
                                            String::new()
                                        };
                                        format!(
                                            "@SerialName(\"{}\") val {}: {}{}",
                                            sn, kt_name, kt_type, default_part
                                        )
                                    })
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                w.line(&format!(
                                    "data class {}({}) : {}()",
                                    name, params, meta.name
                                ));
                            }
                            continue;
                        }
                    }
                }

                if tys.len() == 1 {
                    let kt_type = map_type(&tys[0], &ctx.custom_types, &ctx.known_types);
                    w.line(&format!(
                        "data class {}(val value: {}) : {}()",
                        name, kt_type, meta.name
                    ));
                } else {
                    let params = tys
                        .iter()
                        .enumerate()
                        .map(|(i, ty)| {
                            format!(
                                "val value{}: {}",
                                i,
                                map_type(ty, &ctx.custom_types, &ctx.known_types)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    w.line(&format!(
                        "data class {}({}) : {}()",
                        name, params, meta.name
                    ));
                }
            }
            FfiVariant::Struct {
                name,
                fields,
                serde_rename_all,
            } => {
                let serial_name = apply_rename(name, meta.serde_rename_all.as_deref());
                let field_rename = serde_rename_all
                    .as_deref()
                    .or(meta.serde_rename_all.as_deref());
                let params = fields
                    .iter()
                    .map(|f| {
                        let kt_name = f.name.to_lower_camel_case();
                        let kt_type = map_type(&f.ty, &ctx.custom_types, &ctx.known_types);
                        let sn = apply_rename(&f.name, field_rename);
                        let default_part = if f.has_serde_default {
                            format!(" = {}", resolve_default(f, &kt_type, ctx))
                        } else {
                            String::new()
                        };
                        format!(
                            "@SerialName(\"{}\") val {}: {}{}",
                            sn, kt_name, kt_type, default_part
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                w.line("");
                w.line(&format!("@Serializable @SerialName(\"{}\")", serial_name));
                if fields.is_empty() {
                    w.line(&format!("data object {} : {}()", name, meta.name));
                } else {
                    w.line(&format!(
                        "data class {}({}) : {}()",
                        name, params, meta.name
                    ));
                }
            }
        }
    }
    w.close_block();
    w.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_custom_types() -> HashMap<String, String> {
        HashMap::new()
    }

    fn empty_known_types() -> HashSet<&'static str> {
        HashSet::new()
    }

    fn test_context<'a>(extra_metas: &[&'a FfiMeta]) -> CodegenContext<'a> {
        let mut meta_map = HashMap::new();
        for m in extra_metas {
            meta_map.insert(m.name.as_str(), *m);
        }
        let known_types: HashSet<&str> = meta_map.keys().copied().collect();
        CodegenContext {
            custom_types: HashMap::new(),
            meta_map,
            known_types,
            inlined_types: HashSet::new(),
        }
    }

    #[test]
    fn map_primitives() {
        let ct = empty_custom_types();
        let kt = empty_known_types();
        assert_eq!(map_type("bool", &ct, &kt), "Boolean");
        assert_eq!(map_type("u32", &ct, &kt), "Int");
        assert_eq!(map_type("i64", &ct, &kt), "Long");
        assert_eq!(map_type("usize", &ct, &kt), "Long");
        assert_eq!(map_type("f32", &ct, &kt), "Float");
        assert_eq!(map_type("String", &ct, &kt), "String");
    }

    #[test]
    fn map_option() {
        let ct = empty_custom_types();
        let kt = empty_known_types();
        assert_eq!(map_type("Option<String>", &ct, &kt), "String?");
        assert_eq!(map_type("Option<u32>", &ct, &kt), "Int?");
    }

    #[test]
    fn map_vec() {
        let ct = empty_custom_types();
        let kt = empty_known_types();
        assert_eq!(map_type("Vec<u32>", &ct, &kt), "List<Int>");
        assert_eq!(map_type("Vec<String>", &ct, &kt), "List<String>");
    }

    #[test]
    fn map_imbl_vector() {
        let ct = empty_custom_types();
        let kt = empty_known_types();
        assert_eq!(map_type("imbl::Vector<NodeId>", &ct, &kt), "List<NodeId>");
    }

    #[test]
    fn map_custom_type() {
        let mut ct = HashMap::new();
        ct.insert("NodeId".into(), "String".into());
        let kt = empty_known_types();
        assert_eq!(map_type("NodeId", &ct, &kt), "String");
        assert_eq!(map_type("Option<NodeId>", &ct, &kt), "String?");
        assert_eq!(map_type("Vec<NodeId>", &ct, &kt), "List<String>");
    }

    #[test]
    fn map_unknown_type_passthrough() {
        let ct = empty_custom_types();
        let kt = empty_known_types();
        assert_eq!(map_type("Affinity", &ct, &kt), "Affinity");
        assert_eq!(map_type("Position", &ct, &kt), "Position");
    }

    #[test]
    fn map_known_type_uses_fqn() {
        let ct = empty_custom_types();
        let mut kt = HashSet::new();
        kt.insert("Intent");
        kt.insert("Break");
        assert_eq!(map_type("Intent", &ct, &kt), "co.typie.editor.ffi.Intent");
        assert_eq!(map_type("Break", &ct, &kt), "co.typie.editor.ffi.Break");
        assert_eq!(
            map_type("Option<Intent>", &ct, &kt),
            "co.typie.editor.ffi.Intent?"
        );
    }

    #[test]
    fn map_nested_generic() {
        let ct = empty_custom_types();
        let kt = empty_known_types();
        assert_eq!(map_type("Option<Vec<u32>>", &ct, &kt), "List<Int>?");
        assert_eq!(map_type("Vec<Option<String>>", &ct, &kt), "List<String?>");
    }

    #[test]
    fn map_hashmap_types() {
        let ct = empty_custom_types();
        let kt = empty_known_types();
        assert_eq!(
            map_type("HashMap<String, u32>", &ct, &kt),
            "Map<String, Int>"
        );
        assert_eq!(
            map_type("std::collections::HashMap<String, Vec<u32>>", &ct, &kt),
            "Map<String, List<Int>>"
        );
        assert_eq!(
            map_type("hashbrown::HashMap<String, bool>", &ct, &kt),
            "Map<String, Boolean>"
        );
        assert_eq!(
            map_type("imbl::HashMap<String, f64>", &ct, &kt),
            "Map<String, Double>"
        );
    }

    #[test]
    fn generate_simple_struct() {
        let meta = FfiMeta {
            name: "Size".into(),
            serde_rename_all: None,
            kind: FfiKind::Struct {
                fields: vec![
                    FfiField {
                        name: "width".into(),
                        ty: "f32".into(),
                        has_serde_default: false,
                        ffi_default_override: None,
                    },
                    FfiField {
                        name: "height".into(),
                        ty: "f32".into(),
                        has_serde_default: false,
                        ffi_default_override: None,
                    },
                ],
            },
        };
        let ctx = test_context(&[]);
        let output = generate_data_class(
            &meta,
            match &meta.kind {
                FfiKind::Struct { fields } => fields,
                _ => unreachable!(),
            },
            &ctx,
        );
        assert!(output.contains("package co.typie.editor.ffi"));
        assert!(output.contains("import kotlinx.serialization.SerialName"));
        assert!(output.contains("import kotlinx.serialization.Serializable"));
        assert!(output.contains("@Serializable"));
        assert!(output.contains("data class Size("));
        assert!(output.contains("@SerialName(\"width\") val width: Float,"));
        assert!(output.contains("@SerialName(\"height\") val height: Float,"));
    }

    #[test]
    fn generate_struct_with_custom_type() {
        let mut ctx = test_context(&[]);
        ctx.custom_types.insert("NodeId".into(), "String".into());
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
                        has_serde_default: false,
                        ffi_default_override: None,
                    },
                ],
            },
        };
        let output = generate_data_class(
            &meta,
            match &meta.kind {
                FfiKind::Struct { fields } => fields,
                _ => unreachable!(),
            },
            &ctx,
        );
        assert!(output.contains("@SerialName(\"node_id\") val nodeId: String,"));
        assert!(output.contains("@SerialName(\"offset\") val offset: Long,"));
        assert!(output.contains("@SerialName(\"affinity\") val affinity: Affinity,"));
    }

    #[test]
    fn generate_untagged_unit_enum() {
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
        let ctx = test_context(&[]);
        let output = generate_enum_class(
            &meta,
            match &meta.kind {
                FfiKind::Enum { variants, .. } => variants,
                _ => unreachable!(),
            },
            &ctx,
        );
        assert!(output.contains("package co.typie.editor.ffi"));
        assert!(output.contains("@Serializable"));
        assert!(output.contains("enum class Affinity {"));
        assert!(output.contains("@SerialName(\"downstream\") Downstream,"));
        assert!(output.contains("@SerialName(\"upstream\") Upstream,"));
    }

    #[test]
    fn generate_tagged_enum_sealed_class() {
        let meta = FfiMeta {
            name: "EditorEvent".into(),
            serde_rename_all: Some("snake_case".into()),
            kind: FfiKind::Enum {
                variants: vec![
                    FfiVariant::Struct {
                        name: "StateChanged".into(),
                        fields: vec![FfiField {
                            name: "fields".into(),
                            ty: "Vec<StateField>".into(),
                            has_serde_default: false,
                            ffi_default_override: None,
                        }],
                        serde_rename_all: None,
                    },
                    FfiVariant::Unit {
                        name: "RenderInvalidated".into(),
                    },
                ],
                serde_tag: Some("type".into()),
                default_variant: None,
            },
        };
        let ctx = test_context(&[]);
        let output = generate_sealed_class(
            &meta,
            match &meta.kind {
                FfiKind::Enum { variants, .. } => variants,
                _ => unreachable!(),
            },
            &ctx,
        );
        assert!(output.contains("@Serializable"));
        assert!(output.contains("sealed class EditorEvent {"));
        assert!(output.contains("@Serializable @SerialName(\"state_changed\")"));
        assert!(output.contains(
            "data class StateChanged(@SerialName(\"fields\") val fields: List<StateField>) : EditorEvent()"
        ));
        assert!(output.contains("@Serializable @SerialName(\"render_invalidated\")"));
        assert!(output.contains("data object RenderInvalidated : EditorEvent()"));
    }

    #[test]
    fn generate_unit_only_enum() {
        let meta = FfiMeta {
            name: "Affinity".into(),
            serde_rename_all: None,
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
                default_variant: None,
            },
        };
        let ctx = test_context(&[]);
        let output = generate_enum_class(
            &meta,
            match &meta.kind {
                FfiKind::Enum { variants, .. } => variants,
                _ => unreachable!(),
            },
            &ctx,
        );
        assert!(output.contains("sealed interface Affinity {") == false);
        assert!(output.contains("enum class Affinity {"));
        assert!(output.contains("@SerialName(\"Downstream\") Downstream,"));
        assert!(output.contains("@SerialName(\"Upstream\") Upstream,"));
    }

    #[test]
    fn generate_mixed_enum() {
        let meta = FfiMeta {
            name: "Modifier".into(),
            serde_rename_all: None,
            kind: FfiKind::Enum {
                variants: vec![
                    FfiVariant::Unit {
                        name: "Bold".into(),
                    },
                    FfiVariant::Tuple {
                        name: "FontSize".into(),
                        tys: vec!["u32".into()],
                    },
                    FfiVariant::Struct {
                        name: "Link".into(),
                        fields: vec![FfiField {
                            name: "href".into(),
                            ty: "String".into(),
                            has_serde_default: false,
                            ffi_default_override: None,
                        }],
                        serde_rename_all: None,
                    },
                ],
                serde_tag: Some("type".into()),
                default_variant: None,
            },
        };
        let ctx = test_context(&[]);
        let output = generate_sealed_class(
            &meta,
            match &meta.kind {
                FfiKind::Enum { variants, .. } => variants,
                _ => unreachable!(),
            },
            &ctx,
        );
        assert!(output.contains("sealed class Modifier {"));
        assert!(output.contains("data object Bold : Modifier()"));
        assert!(output.contains("data class FontSize(val value: Int) : Modifier()"));
        assert!(
            output.contains("data class Link(@SerialName(\"href\") val href: String) : Modifier()")
        );
    }

    #[test]
    fn generate_multi_field_tuple_variant() {
        let meta = FfiMeta {
            name: "TestEnum".into(),
            serde_rename_all: None,
            kind: FfiKind::Enum {
                variants: vec![FfiVariant::Tuple {
                    name: "Range".into(),
                    tys: vec!["u32".into(), "u32".into()],
                }],
                serde_tag: Some("type".into()),
                default_variant: None,
            },
        };
        let ctx = test_context(&[]);
        let output = generate_sealed_class(
            &meta,
            match &meta.kind {
                FfiKind::Enum { variants, .. } => variants,
                _ => unreachable!(),
            },
            &ctx,
        );
        assert!(output.contains("data class Range(val value0: Int, val value1: Int) : TestEnum()"));
    }

    #[test]
    fn generate_struct_with_defaults() {
        let affinity_meta = FfiMeta {
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
        let ctx = test_context(&[&affinity_meta]);
        let meta = FfiMeta {
            name: "Position".into(),
            serde_rename_all: Some("snake_case".into()),
            kind: FfiKind::Struct {
                fields: vec![
                    FfiField {
                        name: "node_id".into(),
                        ty: "String".into(),
                        has_serde_default: false,
                        ffi_default_override: None,
                    },
                    FfiField {
                        name: "offset".into(),
                        ty: "usize".into(),
                        has_serde_default: true,
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
        let output = generate_data_class(
            &meta,
            match &meta.kind {
                FfiKind::Struct { fields } => fields,
                _ => unreachable!(),
            },
            &ctx,
        );
        assert!(output.contains("@SerialName(\"node_id\") val nodeId: String,"));
        assert!(output.contains("@SerialName(\"offset\") val offset: Long = 0L,"));
        assert!(
            output.contains(
                "@SerialName(\"affinity\") val affinity: co.typie.editor.ffi.Affinity = co.typie.editor.ffi.Affinity.Downstream,"
            )
        );
    }

    #[test]
    fn generate_struct_with_ffi_default_override() {
        let ctx = test_context(&[]);
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
        let output = generate_data_class(
            &meta,
            match &meta.kind {
                FfiKind::Struct { fields } => fields,
                _ => unreachable!(),
            },
            &ctx,
        );
        assert!(output.contains("@SerialName(\"proportion\") val proportion: Float = 1.0f,"));
    }

    #[test]
    fn generate_tagged_enum_with_newtype_flattening() {
        let inner_meta = FfiMeta {
            name: "StateChangedPayload".into(),
            serde_rename_all: Some("snake_case".into()),
            kind: FfiKind::Struct {
                fields: vec![
                    FfiField {
                        name: "doc_version".into(),
                        ty: "u64".into(),
                        has_serde_default: false,
                        ffi_default_override: None,
                    },
                    FfiField {
                        name: "fields".into(),
                        ty: "Vec<String>".into(),
                        has_serde_default: false,
                        ffi_default_override: None,
                    },
                ],
            },
        };
        let ctx = test_context(&[&inner_meta]);
        let meta = FfiMeta {
            name: "EditorEvent".into(),
            serde_rename_all: Some("snake_case".into()),
            kind: FfiKind::Enum {
                variants: vec![
                    FfiVariant::Tuple {
                        name: "StateChanged".into(),
                        tys: vec!["StateChangedPayload".into()],
                    },
                    FfiVariant::Unit {
                        name: "RenderInvalidated".into(),
                    },
                ],
                serde_tag: Some("type".into()),
                default_variant: None,
            },
        };
        let output = generate_sealed_class(
            &meta,
            match &meta.kind {
                FfiKind::Enum { variants, .. } => variants,
                _ => unreachable!(),
            },
            &ctx,
        );
        assert!(output.contains("sealed class EditorEvent {"));
        assert!(output.contains("@SerialName(\"doc_version\") val docVersion: Long"));
        assert!(output.contains("@SerialName(\"fields\") val fields: List<String>"));
        assert!(!output.contains("val value:"));
        assert!(output.contains("data object RenderInvalidated : EditorEvent()"));
    }
}
