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
    let generics = input
        .generics
        .params
        .iter()
        .filter_map(|p| match p {
            syn::GenericParam::Type(t) => Some(t.ident.to_string()),
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
            let variants = data.variants.iter().map(extract_variant).collect();
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

fn extract_variant(v: &syn::Variant) -> FfiVariant {
    let vname = v.ident.to_string();
    match &v.fields {
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
    }
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
