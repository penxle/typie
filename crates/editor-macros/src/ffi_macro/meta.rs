use std::collections::HashSet;

use editor_bindgen::meta::{FfiField, FfiKind, FfiMeta, FfiVariant};
use quote::quote;
use syn::DeriveInput;

const UNSUPPORTED_FIELD_ATTRS: &[&str] = &[
    "flatten",
    "serialize_with",
    "deserialize_with",
    "skip_serializing",
    "skip_deserializing",
];

pub(super) fn type_to_string(ty: &syn::Type) -> String {
    quote!(#ty)
        .to_string()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("")
}

pub fn extract(input: &DeriveInput, custom: Option<&syn::Type>) -> syn::Result<FfiMeta> {
    let name = input.ident.to_string();
    let used = collect_used_idents(input, custom);
    let generics = input
        .generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(t) => {
                let ident = t.ident.to_string();
                used.contains(&ident).then_some(ident)
            }
            _ => None,
        })
        .collect();

    if let Some(custom) = custom {
        return Ok(FfiMeta {
            name,
            serde_rename_all: None,
            kind: FfiKind::Custom {
                target: type_to_string(custom),
            },
            generics,
        });
    }

    reject_serde_attrs(
        &input.attrs,
        &[
            "rename_all_fields",
            "untagged",
            "transparent",
            "from",
            "into",
            "try_from",
            "variant_identifier",
            "field_identifier",
        ],
        "",
    )?;
    let serde_rename_all = parse_serde_string(&input.attrs, "rename_all")?;
    let kind = match &input.data {
        syn::Data::Struct(data) => {
            match &data.fields {
                syn::Fields::Named(_) => {}
                syn::Fields::Unnamed(fields) => {
                    return Err(syn::Error::new_spanned(
                        fields,
                        "#[ffi] does not support tuple structs",
                    ));
                }
                syn::Fields::Unit => {
                    return Err(syn::Error::new_spanned(
                        &input.ident,
                        "#[ffi] does not support unit structs",
                    ));
                }
            }
            for field in &data.fields {
                reject_unsupported_field_attrs(&field.attrs, " on struct fields")?;
            }
            let fields = data
                .fields
                .iter()
                .filter(|f| !has_serde_skip_attr(&f.attrs))
                .map(extract_field)
                .collect::<syn::Result<_>>()?;
            FfiKind::Struct { fields }
        }
        syn::Data::Enum(data) => {
            let serde_tag = parse_serde_string(&input.attrs, "tag")?;
            let serde_content = parse_serde_string(&input.attrs, "content")?;
            let default_variant = find_default_variant(&data.variants);
            let variants = data
                .variants
                .iter()
                .filter_map(|v| extract_variant(v).transpose())
                .collect::<syn::Result<_>>()?;
            FfiKind::Enum {
                variants,
                serde_tag,
                serde_content,
                default_variant,
            }
        }
        syn::Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "#[ffi] does not support unions",
            ));
        }
    };

    Ok(FfiMeta {
        name,
        serde_rename_all,
        kind,
        generics,
    })
}

fn collect_used_idents(input: &DeriveInput, custom: Option<&syn::Type>) -> HashSet<String> {
    let mut used = HashSet::new();
    if let Some(ty) = custom {
        collect_type_idents(ty, &mut used);
        return used;
    }
    match &input.data {
        syn::Data::Struct(data) => {
            for field in &data.fields {
                if !has_serde_skip_attr(&field.attrs) {
                    collect_type_idents(&field.ty, &mut used);
                }
            }
        }
        syn::Data::Enum(data) => {
            for variant in &data.variants {
                if has_ffi_skip_attr(&variant.attrs) {
                    continue;
                }
                for field in &variant.fields {
                    if !has_serde_skip_attr(&field.attrs) {
                        collect_type_idents(&field.ty, &mut used);
                    }
                }
            }
        }
        syn::Data::Union(_) => {}
    }
    used
}

fn collect_type_idents(ty: &syn::Type, out: &mut HashSet<String>) {
    match ty {
        syn::Type::Path(p) => {
            for seg in &p.path.segments {
                out.insert(seg.ident.to_string());
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(t) = arg {
                            collect_type_idents(t, out);
                        }
                    }
                }
            }
        }
        syn::Type::Tuple(t) => t.elems.iter().for_each(|e| collect_type_idents(e, out)),
        syn::Type::Reference(r) => collect_type_idents(&r.elem, out),
        syn::Type::Array(a) => collect_type_idents(&a.elem, out),
        syn::Type::Slice(s) => collect_type_idents(&s.elem, out),
        syn::Type::Paren(p) => collect_type_idents(&p.elem, out),
        syn::Type::Group(g) => collect_type_idents(&g.elem, out),
        _ => {}
    }
}

fn extract_field(f: &syn::Field) -> syn::Result<FfiField> {
    let Some(name) = &f.ident else {
        return Err(syn::Error::new_spanned(
            &f.ty,
            "#[ffi] does not support tuple structs",
        ));
    };
    let serde_rename = parse_serde_string(&f.attrs, "rename")?;
    let has_serde_default = has_serde_default_attr(&f.attrs);
    let ffi_default_override = parse_ffi_default(&f.attrs);
    Ok(FfiField {
        name: name.to_string(),
        serde_rename,
        ty: type_to_string(&f.ty),
        has_serde_default,
        ffi_default_override,
    })
}

pub(super) fn has_ffi_skip_attr(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("ffi") {
            continue;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                found = true;
            }
            Ok(())
        });
        if found {
            return true;
        }
    }
    false
}

fn extract_variant(v: &syn::Variant) -> syn::Result<Option<FfiVariant>> {
    if has_ffi_skip_attr(&v.attrs) {
        return Ok(None);
    }
    reject_serde_attrs(
        &v.attrs,
        &[
            "rename",
            "untagged",
            "serialize_with",
            "deserialize_with",
            "with",
        ],
        " on enum variants",
    )?;
    reject_serde_attrs(
        &v.attrs,
        &["skip", "skip_serializing", "skip_deserializing"],
        " on enum variants; use #[ffi(skip)] to hide the variant from bindings",
    )?;
    let vname = v.ident.to_string();
    Ok(Some(match &v.fields {
        syn::Fields::Unit => FfiVariant::Unit { name: vname },
        syn::Fields::Unnamed(fields) => {
            for field in &fields.unnamed {
                reject_serde_attrs(&field.attrs, &["skip"], " on tuple variant fields")?;
                reject_unsupported_field_attrs(&field.attrs, " on tuple variant fields")?;
            }
            FfiVariant::Tuple {
                name: vname,
                tys: fields
                    .unnamed
                    .iter()
                    .map(|f| type_to_string(&f.ty))
                    .collect(),
            }
        }
        syn::Fields::Named(fields) => {
            for field in &fields.named {
                reject_serde_attrs(&field.attrs, &["skip"], " on struct variant fields")?;
                reject_unsupported_field_attrs(&field.attrs, " on struct variant fields")?;
            }
            let serde_rename_all = parse_serde_string(&v.attrs, "rename_all")?;
            FfiVariant::Struct {
                name: vname,
                fields: fields
                    .named
                    .iter()
                    .map(extract_field)
                    .collect::<syn::Result<_>>()?,
                serde_rename_all,
            }
        }
    }))
}

fn parse_serde_string(attrs: &[syn::Attribute], key: &str) -> syn::Result<Option<String>> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let mut result = None;
        attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident(key) {
                return skip_meta_value(&meta);
            }
            if meta.input.peek(syn::token::Paren) {
                return Err(syn::Error::new_spanned(
                    attr,
                    format!("#[ffi] does not support `#[serde({key}(...))]`"),
                ));
            }
            let lit: syn::LitStr = meta.value()?.parse()?;
            result = Some(lit.value());
            Ok(())
        })?;
        if result.is_some() {
            return Ok(result);
        }
    }
    Ok(None)
}

fn has_serde_skip_attr(attrs: &[syn::Attribute]) -> bool {
    find_serde_attr(attrs, "skip").is_some()
}

fn reject_serde_attrs(attrs: &[syn::Attribute], keys: &[&str], suffix: &str) -> syn::Result<()> {
    for key in keys {
        if let Some(attr) = find_serde_attr(attrs, key) {
            return Err(syn::Error::new_spanned(
                attr,
                format!("#[ffi] does not support `#[serde({key})]`{suffix}"),
            ));
        }
    }
    Ok(())
}

fn reject_unsupported_field_attrs(attrs: &[syn::Attribute], suffix: &str) -> syn::Result<()> {
    reject_serde_attrs(attrs, UNSUPPORTED_FIELD_ATTRS, suffix)?;
    if let Some(attr) = find_serde_attr(attrs, "with")
        && let Some(module) = parse_serde_string(std::slice::from_ref(attr), "with")?
        && module != "serde_bytes"
    {
        return Err(syn::Error::new_spanned(
            attr,
            format!(
                "#[ffi] does not support `#[serde(with = \"{module}\")]`{suffix}; only `serde_bytes` is supported"
            ),
        ));
    }
    Ok(())
}

fn find_serde_attr<'a>(attrs: &'a [syn::Attribute], key: &str) -> Option<&'a syn::Attribute> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("serde"))
        .find(|attr| {
            let mut found = false;
            let _ = attr.parse_nested_meta(|meta| {
                found |= meta.path.is_ident(key);
                skip_meta_value(&meta)
            });
            found
        })
}

fn skip_meta_value(meta: &syn::meta::ParseNestedMeta) -> syn::Result<()> {
    if meta.input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in meta.input);
        content.parse::<proc_macro2::TokenStream>()?;
    } else if meta.input.peek(syn::Token![=]) {
        meta.value()?.parse::<syn::Expr>()?;
    }
    Ok(())
}

fn has_serde_default_attr(attrs: &[syn::Attribute]) -> bool {
    find_serde_attr(attrs, "default").is_some()
}

/// Parse `#[ffi(default = "...")]` from attributes.
fn parse_ffi_default(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("ffi") {
            continue;
        }
        let mut result = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                let value = meta.value()?;
                let lit: syn::LitStr = value.parse()?;
                result = Some(lit.value());
            }
            Ok(())
        });
        if result.is_some() {
            return result;
        }
    }
    None
}

/// Find variant with `#[default]` attribute in an enum.
fn find_default_variant(
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>,
) -> Option<String> {
    for v in variants {
        for attr in &v.attrs {
            if attr.path().is_ident("default") {
                return Some(v.ident.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_rename_is_rejected() {
        let input: DeriveInput = syn::parse_quote! {
            #[serde(tag = "type", rename_all = "snake_case")]
            enum Event {
                #[serde(rename = "other")]
                Changed,
            }
        };
        let error = extract(&input, None).unwrap_err();
        assert_eq!(
            error.to_string(),
            "#[ffi] does not support `#[serde(rename)]` on enum variants"
        );
    }

    #[test]
    fn struct_variant_field_skip_is_rejected() {
        let input: DeriveInput = syn::parse_quote! {
            #[serde(tag = "type", rename_all = "snake_case")]
            enum Event {
                Changed {
                    value: u32,
                    #[serde(skip)]
                    cache: u32,
                },
            }
        };
        let error = extract(&input, None).unwrap_err();
        assert_eq!(
            error.to_string(),
            "#[ffi] does not support `#[serde(skip)]` on struct variant fields"
        );
    }

    #[test]
    fn supported_serde_attributes_are_accepted() {
        let input: DeriveInput = syn::parse_quote! {
            #[serde(tag = "type", rename_all = "snake_case")]
            enum Event {
                Changed {
                    #[serde(rename = "v")]
                    value: u32,
                },
                #[ffi(skip)]
                #[serde(rename = "hidden")]
                Hidden {
                    #[serde(skip)]
                    cache: u32,
                },
            }
        };
        let meta = extract(&input, None).unwrap();
        let FfiKind::Enum { variants, .. } = meta.kind else {
            panic!("expected an enum, got {:?}", meta.kind);
        };
        assert_eq!(variants.len(), 1);

        let input: DeriveInput = syn::parse_quote! {
            struct Position {
                offset: u32,
                #[serde(skip)]
                cache: u32,
            }
        };
        let meta = extract(&input, None).unwrap();
        let FfiKind::Struct { fields } = meta.kind else {
            panic!("expected a struct, got {:?}", meta.kind);
        };
        assert_eq!(
            fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
            ["offset"]
        );
    }

    #[test]
    fn list_metas_before_the_key_do_not_hide_it() {
        let input: DeriveInput = syn::parse_quote! {
            #[serde(tag = "type", rename_all = "snake_case")]
            enum Event {
                #[serde(bound(serialize = "T: Serialize"), rename = "x")]
                Changed,
            }
        };
        let error = extract(&input, None).unwrap_err();
        assert_eq!(
            error.to_string(),
            "#[ffi] does not support `#[serde(rename)]` on enum variants"
        );

        let input: DeriveInput = syn::parse_quote! {
            #[serde(tag = "type", rename_all = "snake_case")]
            enum Event {
                Changed {
                    value: u32,
                    #[serde(bound(serialize = "T: Serialize"), skip)]
                    cache: u32,
                },
            }
        };
        let error = extract(&input, None).unwrap_err();
        assert_eq!(
            error.to_string(),
            "#[ffi] does not support `#[serde(skip)]` on struct variant fields"
        );

        let input: DeriveInput = syn::parse_quote! {
            struct Position {
                offset: u32,
                #[serde(bound(serialize = "T: Serialize"), skip)]
                cache: u32,
            }
        };
        let meta = extract(&input, None).unwrap();
        let FfiKind::Struct { fields } = meta.kind else {
            panic!("expected a struct, got {:?}", meta.kind);
        };
        assert_eq!(
            fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
            ["offset"]
        );
    }

    #[test]
    fn list_form_variant_rename_is_rejected() {
        let input: DeriveInput = syn::parse_quote! {
            #[serde(tag = "type", rename_all = "snake_case")]
            enum Event {
                #[serde(rename(serialize = "a", deserialize = "b"))]
                Changed,
            }
        };
        let error = extract(&input, None).unwrap_err();
        assert_eq!(
            error.to_string(),
            "#[ffi] does not support `#[serde(rename)]` on enum variants"
        );
    }

    #[test]
    fn wire_changing_serde_attributes_are_rejected() {
        let cases: [(DeriveInput, &str); 7] = [
            (
                syn::parse_quote! {
                    #[serde(tag = "type", rename_all_fields = "camelCase")]
                    enum Event {
                        Changed { page_index: u32 },
                    }
                },
                "#[ffi] does not support `#[serde(rename_all_fields)]`",
            ),
            (
                syn::parse_quote! {
                    #[serde(untagged)]
                    enum Value {
                        Number(u32),
                    }
                },
                "#[ffi] does not support `#[serde(untagged)]`",
            ),
            (
                syn::parse_quote! {
                    #[serde(tag = "type")]
                    enum Event {
                        #[serde(skip)]
                        Changed,
                    }
                },
                "#[ffi] does not support `#[serde(skip)]` on enum variants; use #[ffi(skip)] to hide the variant from bindings",
            ),
            (
                syn::parse_quote! {
                    #[serde(tag = "type")]
                    enum Event {
                        #[serde(skip_serializing)]
                        Changed,
                    }
                },
                "#[ffi] does not support `#[serde(skip_serializing)]` on enum variants; use #[ffi(skip)] to hide the variant from bindings",
            ),
            (
                syn::parse_quote! {
                    #[serde(tag = "type")]
                    enum Event {
                        #[serde(skip_deserializing)]
                        Changed,
                    }
                },
                "#[ffi] does not support `#[serde(skip_deserializing)]` on enum variants; use #[ffi(skip)] to hide the variant from bindings",
            ),
            (
                syn::parse_quote! {
                    struct Position {
                        #[serde(flatten)]
                        offset: Offset,
                    }
                },
                "#[ffi] does not support `#[serde(flatten)]` on struct fields",
            ),
            (
                syn::parse_quote! {
                    #[serde(tag = "type")]
                    enum Event {
                        Changed {
                            #[serde(flatten)]
                            offset: Offset,
                        },
                    }
                },
                "#[ffi] does not support `#[serde(flatten)]` on struct variant fields",
            ),
        ];
        for (input, message) in cases {
            let error = extract(&input, None).expect_err(message);
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn alias_other_and_bound_are_accepted() {
        let input: DeriveInput = syn::parse_quote! {
            #[serde(tag = "type", bound(serialize = "T: Serialize"))]
            enum Event {
                #[serde(bound(deserialize = "T: Deserialize"), alias = "edited")]
                Changed {
                    #[serde(alias = "v")]
                    value: u32,
                },
                #[serde(other)]
                Unknown,
            }
        };
        let meta = extract(&input, None).unwrap();
        let FfiKind::Enum { variants, .. } = meta.kind else {
            panic!("expected an enum, got {:?}", meta.kind);
        };
        assert_eq!(variants.len(), 2);
    }

    #[test]
    fn unions_are_rejected() {
        let input: DeriveInput = syn::parse_quote! {
            union Bits {
                value: u32,
            }
        };
        let error = extract(&input, None).unwrap_err();
        assert_eq!(error.to_string(), "#[ffi] does not support unions");
    }

    #[test]
    fn tuple_structs_are_rejected() {
        let input: DeriveInput = syn::parse_quote! {
            struct Offset(u32);
        };
        let error = extract(&input, None).unwrap_err();
        assert_eq!(error.to_string(), "#[ffi] does not support tuple structs");
    }

    #[test]
    fn tuple_structs_with_only_skipped_fields_are_rejected() {
        let input: DeriveInput = syn::parse_quote! {
            struct Offset(#[serde(skip)] u32);
        };
        let error = extract(&input, None).expect_err("tuple struct must be rejected");
        assert_eq!(error.to_string(), "#[ffi] does not support tuple structs");
    }

    fn rejection(input: DeriveInput) -> String {
        extract(&input, None).unwrap_err().to_string()
    }

    #[test]
    fn unit_structs_are_rejected() {
        assert_eq!(
            rejection(syn::parse_quote! {
                struct Marker;
            }),
            "#[ffi] does not support unit structs"
        );
    }

    #[test]
    fn tuple_variant_field_skip_is_rejected() {
        assert_eq!(
            rejection(syn::parse_quote! {
                #[serde(tag = "type", content = "value")]
                enum Event {
                    Changed(#[serde(skip)] u32),
                }
            }),
            "#[ffi] does not support `#[serde(skip)]` on tuple variant fields"
        );
    }

    #[test]
    fn serde_attributes_with_their_own_wire_shape_are_rejected() {
        for (key, meta) in [
            ("transparent", quote!(transparent)),
            ("from", quote!(from = "Offset")),
            ("into", quote!(into = "Offset")),
            ("try_from", quote!(try_from = "Offset")),
        ] {
            assert_eq!(
                rejection(syn::parse_quote! {
                    #[serde(#meta)]
                    struct Position {
                        offset: u32,
                    }
                }),
                format!("#[ffi] does not support `#[serde({key})]`")
            );
        }
        for (key, meta) in [
            ("variant_identifier", quote!(variant_identifier)),
            ("field_identifier", quote!(field_identifier)),
        ] {
            assert_eq!(
                rejection(syn::parse_quote! {
                    #[serde(#meta)]
                    enum Field {
                        Offset,
                        Length,
                    }
                }),
                format!("#[ffi] does not support `#[serde({key})]`")
            );
        }
        for (key, meta) in [
            ("untagged", quote!(untagged)),
            ("serialize_with", quote!(serialize_with = "write_event")),
            ("deserialize_with", quote!(deserialize_with = "read_event")),
            ("with", quote!(with = "event_format")),
        ] {
            assert_eq!(
                rejection(syn::parse_quote! {
                    #[serde(tag = "type")]
                    enum Event {
                        #[serde(#meta)]
                        Changed { value: u32 },
                    }
                }),
                format!("#[ffi] does not support `#[serde({key})]` on enum variants")
            );
        }
        for (key, meta) in [
            ("serialize_with", quote!(serialize_with = "write_offset")),
            ("deserialize_with", quote!(deserialize_with = "read_offset")),
            ("skip_serializing", quote!(skip_serializing)),
            ("skip_deserializing", quote!(skip_deserializing)),
        ] {
            assert_eq!(
                rejection(syn::parse_quote! {
                    struct Position {
                        #[serde(#meta)]
                        offset: u32,
                    }
                }),
                format!("#[ffi] does not support `#[serde({key})]` on struct fields")
            );
            assert_eq!(
                rejection(syn::parse_quote! {
                    #[serde(tag = "type")]
                    enum Event {
                        Changed {
                            #[serde(#meta)]
                            offset: u32,
                        },
                    }
                }),
                format!("#[ffi] does not support `#[serde({key})]` on struct variant fields")
            );
        }
    }

    #[test]
    fn serde_bytes_fields_are_accepted() {
        let input: DeriveInput = syn::parse_quote! {
            struct Chunk {
                #[serde(with = "serde_bytes")]
                bytes: Vec<u8>,
            }
        };
        let meta = extract(&input, None).unwrap();
        let FfiKind::Struct { fields } = meta.kind else {
            panic!("expected a struct, got {:?}", meta.kind);
        };
        assert_eq!(
            fields.iter().map(|f| f.ty.as_str()).collect::<Vec<_>>(),
            ["Vec<u8>"]
        );

        let input: DeriveInput = syn::parse_quote! {
            #[serde(tag = "type", content = "value")]
            enum Event {
                Chunk {
                    #[serde(with = "serde_bytes")]
                    bytes: Vec<u8>,
                },
                Raw(#[serde(with = "serde_bytes")] Vec<u8>),
            }
        };
        let meta = extract(&input, None).unwrap();
        let FfiKind::Enum { variants, .. } = meta.kind else {
            panic!("expected an enum, got {:?}", meta.kind);
        };
        let [
            FfiVariant::Struct { fields, .. },
            FfiVariant::Tuple { tys, .. },
        ] = variants.as_slice()
        else {
            panic!("expected a struct and a tuple variant, got {variants:?}");
        };
        assert_eq!(
            fields.iter().map(|f| f.ty.as_str()).collect::<Vec<_>>(),
            ["Vec<u8>"]
        );
        assert_eq!(tys, &["Vec<u8>"]);
    }

    #[test]
    fn field_with_modules_other_than_serde_bytes_are_rejected() {
        let cases: [(DeriveInput, &str); 3] = [
            (
                syn::parse_quote! {
                    struct Position {
                        #[serde(with = "other")]
                        offset: u32,
                    }
                },
                "#[ffi] does not support `#[serde(with = \"other\")]` on struct fields; only `serde_bytes` is supported",
            ),
            (
                syn::parse_quote! {
                    #[serde(tag = "type")]
                    enum Event {
                        Changed {
                            #[serde(with = "other")]
                            offset: u32,
                        },
                    }
                },
                "#[ffi] does not support `#[serde(with = \"other\")]` on struct variant fields; only `serde_bytes` is supported",
            ),
            (
                syn::parse_quote! {
                    #[serde(tag = "type", content = "value")]
                    enum Event {
                        Changed(#[serde(with = "other")] u32),
                    }
                },
                "#[ffi] does not support `#[serde(with = \"other\")]` on tuple variant fields; only `serde_bytes` is supported",
            ),
        ];
        for (input, message) in cases {
            let error = extract(&input, None).expect_err(message);
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn tuple_variant_fields_reject_unsupported_field_attributes() {
        for (key, meta) in [
            ("serialize_with", quote!(serialize_with = "write_offset")),
            ("deserialize_with", quote!(deserialize_with = "read_offset")),
            ("skip_serializing", quote!(skip_serializing)),
            ("skip_deserializing", quote!(skip_deserializing)),
            ("flatten", quote!(flatten)),
        ] {
            let input: DeriveInput = syn::parse_quote! {
                #[serde(tag = "type", content = "value")]
                enum Event {
                    Changed(#[serde(#meta)] u32),
                }
            };
            let message =
                format!("#[ffi] does not support `#[serde({key})]` on tuple variant fields");
            let error = extract(&input, None).expect_err(&message);
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn list_metas_before_string_keys_do_not_hide_them() {
        let input: DeriveInput = syn::parse_quote! {
            #[serde(bound(serialize = "T: Serialize"), tag = "type")]
            #[serde(bound(deserialize = "T: Deserialize"), rename_all = "camelCase")]
            enum Event {
                Changed,
            }
        };
        let meta = extract(&input, None).unwrap();
        assert_eq!(meta.serde_rename_all.as_deref(), Some("camelCase"));
        let FfiKind::Enum { serde_tag, .. } = meta.kind else {
            panic!("expected an enum, got {:?}", meta.kind);
        };
        assert_eq!(serde_tag.as_deref(), Some("type"));

        let input: DeriveInput = syn::parse_quote! {
            struct Position {
                #[serde(bound(serialize = "T: Serialize"), rename = "x")]
                offset: u32,
                #[serde(bound(deserialize = "T: Deserialize"), default)]
                length: u32,
            }
        };
        let meta = extract(&input, None).unwrap();
        let FfiKind::Struct { fields } = meta.kind else {
            panic!("expected a struct, got {:?}", meta.kind);
        };
        assert_eq!(fields[0].serde_rename.as_deref(), Some("x"));
        assert!(fields[1].has_serde_default);
    }

    #[test]
    fn list_form_string_keys_are_rejected() {
        let cases: [(DeriveInput, &str); 3] = [
            (
                syn::parse_quote! {
                    #[serde(rename_all(serialize = "camelCase", deserialize = "camelCase"))]
                    struct Position {
                        offset: u32,
                    }
                },
                "#[ffi] does not support `#[serde(rename_all(...))]`",
            ),
            (
                syn::parse_quote! {
                    struct Position {
                        #[serde(rename(serialize = "a", deserialize = "b"))]
                        offset: u32,
                    }
                },
                "#[ffi] does not support `#[serde(rename(...))]`",
            ),
            (
                syn::parse_quote! {
                    #[serde(tag = "type")]
                    enum Event {
                        #[serde(rename_all(serialize = "camelCase"))]
                        Changed { page_index: u32 },
                    }
                },
                "#[ffi] does not support `#[serde(rename_all(...))]`",
            ),
        ];
        for (input, message) in cases {
            let error = extract(&input, None).expect_err(message);
            assert_eq!(error.to_string(), message);
        }
    }
}
