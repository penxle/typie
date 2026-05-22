use std::collections::HashSet;

use editor_bindgen::meta::{FfiField, FfiKind, FfiMeta, FfiVariant};
use quote::quote;
use syn::DeriveInput;

pub(super) fn type_to_string(ty: &syn::Type) -> String {
    quote!(#ty)
        .to_string()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("")
}

pub fn extract(input: &DeriveInput, custom: Option<&syn::Type>) -> FfiMeta {
    let name = input.ident.to_string();
    let serde_rename_all = parse_serde_rename_all(&input.attrs);
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
        return FfiMeta {
            name,
            serde_rename_all: None,
            kind: FfiKind::Custom {
                target: type_to_string(custom),
            },
            generics,
        };
    }

    let kind = match &input.data {
        syn::Data::Struct(data) => {
            let fields = data
                .fields
                .iter()
                .filter(|f| !has_serde_skip_attr(&f.attrs))
                .map(extract_field)
                .collect();
            FfiKind::Struct { fields }
        }
        syn::Data::Enum(data) => {
            let serde_tag = parse_serde_tag(&input.attrs);
            let default_variant = find_default_variant(&data.variants);
            let variants = data.variants.iter().filter_map(extract_variant).collect();
            FfiKind::Enum {
                variants,
                serde_tag,
                default_variant,
            }
        }
        syn::Data::Union(_) => panic!("#[ffi] does not support unions"),
    };

    FfiMeta {
        name,
        serde_rename_all,
        kind,
        generics,
    }
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

fn extract_field(f: &syn::Field) -> FfiField {
    let serde_rename = parse_serde_rename(&f.attrs);
    let has_serde_default = has_serde_default_attr(&f.attrs);
    let ffi_default_override = parse_ffi_default(&f.attrs);
    FfiField {
        name: f.ident.as_ref().expect("named field").to_string(),
        serde_rename,
        ty: type_to_string(&f.ty),
        has_serde_default,
        ffi_default_override,
    }
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

fn extract_variant(v: &syn::Variant) -> Option<FfiVariant> {
    if has_ffi_skip_attr(&v.attrs) {
        return None;
    }
    let vname = v.ident.to_string();
    Some(match &v.fields {
        syn::Fields::Unit => FfiVariant::Unit { name: vname },
        syn::Fields::Unnamed(fields) => FfiVariant::Tuple {
            name: vname,
            tys: fields
                .unnamed
                .iter()
                .map(|f| type_to_string(&f.ty))
                .collect(),
        },
        syn::Fields::Named(fields) => {
            let serde_rename_all = parse_serde_rename_all(&v.attrs);
            FfiVariant::Struct {
                name: vname,
                fields: fields.named.iter().map(extract_field).collect(),
                serde_rename_all,
            }
        }
    })
}

/// Parse `#[serde(rename = "...")]` from attributes.
fn parse_serde_rename(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let mut result = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let lit: syn::LitStr = value.parse()?;
                result = Some(lit.value());
            } else if meta.input.peek(syn::Token![=]) {
                let _value = meta.value()?;
                let _lit: syn::LitStr = _value.parse()?;
            }
            Ok(())
        });
        if result.is_some() {
            return result;
        }
    }
    None
}

/// Parse `#[serde(rename_all = "...")]` from attributes.
fn parse_serde_rename_all(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let mut result = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                let value = meta.value()?;
                let lit: syn::LitStr = value.parse()?;
                result = Some(lit.value());
            } else if meta.input.peek(syn::Token![=]) {
                // Consume value of other keys (e.g. tag = "type") to avoid parse errors
                let _value = meta.value()?;
                let _lit: syn::LitStr = _value.parse()?;
            }
            Ok(())
        });
        if result.is_some() {
            return result;
        }
    }
    None
}

/// Parse `#[serde(tag = "...")]` from attributes.
fn parse_serde_tag(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let mut result = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("tag") {
                let value = meta.value()?;
                let lit: syn::LitStr = value.parse()?;
                result = Some(lit.value());
            } else if meta.input.peek(syn::Token![=]) {
                let _value = meta.value()?;
                let _lit: syn::LitStr = _value.parse()?;
            }
            Ok(())
        });
        if result.is_some() {
            return result;
        }
    }
    None
}

/// Check for `#[serde(skip)]` on a field.
fn has_serde_skip_attr(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                found = true;
            } else if meta.input.peek(syn::Token![=]) {
                let _value = meta.value()?;
                let _lit: syn::LitStr = _value.parse()?;
            }
            Ok(())
        });
        if found {
            return true;
        }
    }
    false
}

/// Check for `#[serde(default)]` or `#[serde(default = "...")]` on a field.
fn has_serde_default_attr(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                found = true;
                // Consume optional `= "fn_name"` if present
                if meta.input.peek(syn::Token![=]) {
                    let _value = meta.value()?;
                    let _lit: syn::LitStr = _value.parse()?;
                }
            } else if meta.input.peek(syn::Token![=]) {
                let _value = meta.value()?;
                let _lit: syn::LitStr = _value.parse()?;
            }
            Ok(())
        });
        if found {
            return true;
        }
    }
    false
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
