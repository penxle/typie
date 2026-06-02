use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{
    Attribute, Data, DeriveInput, Fields, GenericArgument, LitStr, Meta, PathArguments, Token,
    Type, punctuated::Punctuated,
};

pub enum FieldKind {
    LwwReg { inner: Type },
    OrMap { key: Type, value: Type },
    OrSet { elem: Type },
}

pub struct FieldSpec {
    pub name: syn::Ident,
    pub variant: syn::Ident,
    pub kind: FieldKind,
    pub default: Option<TokenStream>,
    pub plain_attrs: Vec<Meta>,
}

pub struct NodeAttrInput {
    pub struct_ident: syn::Ident,
    pub attr_ident: syn::Ident,
    pub plain_ident: syn::Ident,
    pub fields: Vec<FieldSpec>,
}

impl NodeAttrInput {
    pub fn from_derive(derive: &DeriveInput) -> syn::Result<Self> {
        let struct_ident = derive.ident.clone();
        let struct_name = struct_ident.to_string();
        if !struct_name.ends_with("Node") {
            return Err(syn::Error::new_spanned(
                &derive.ident,
                "NodeAttr expects an ident ending in `Node` (e.g. CalloutNode)",
            ));
        }

        let attr_ident = format_ident!("{}Attr", struct_ident);
        let plain_ident = format_ident!("Plain{}", struct_name);

        let named = match &derive.data {
            Data::Struct(s) => match &s.fields {
                Fields::Named(named) => &named.named,
                _ => {
                    return Err(syn::Error::new_spanned(
                        &derive.ident,
                        "NodeAttr requires named fields",
                    ));
                }
            },
            _ => {
                return Err(syn::Error::new_spanned(
                    &derive.ident,
                    "NodeAttr only applies to structs",
                ));
            }
        };

        let mut fields: Vec<FieldSpec> = Vec::new();
        for f in named.iter() {
            let name = match f.ident.as_ref() {
                Some(n) => n.clone(),
                None => continue,
            };
            let kind = match detect_kind(&f.ty) {
                Some(k) => k,
                None => {
                    return Err(syn::Error::new_spanned(
                        f,
                        format!(
                            "NodeAttr field `{}` must be `LwwReg<T>`, `OrMap<K, V>`, or `OrSet<T>`",
                            name
                        ),
                    ));
                }
            };
            let variant = format_ident!("{}", heck::AsPascalCase(name.to_string()).to_string());
            let default = parse_default_attr(&f.attrs)?;
            if default.is_some() && !matches!(kind, FieldKind::LwwReg { .. }) {
                return Err(syn::Error::new_spanned(
                    f,
                    "`#[node_attr(default = ...)]` is only supported on `LwwReg<T>` fields",
                ));
            }
            let plain_attrs = parse_plain_attrs(&f.attrs)?;
            fields.push(FieldSpec {
                name,
                variant,
                kind,
                default,
                plain_attrs,
            });
        }

        Ok(Self {
            struct_ident,
            attr_ident,
            plain_ident,
            fields,
        })
    }
}

fn detect_kind(ty: &Type) -> Option<FieldKind> {
    let Type::Path(tp) = ty else { return None };
    let last = tp.path.segments.last()?;
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    let ident = last.ident.to_string();
    match (ident.as_str(), args.args.len()) {
        ("LwwReg", 1) => {
            let inner = match args.args.first()? {
                GenericArgument::Type(t) => t.clone(),
                _ => return None,
            };
            Some(FieldKind::LwwReg { inner })
        }
        ("OrMap", 2) => {
            let mut iter = args.args.iter();
            let key = match iter.next()? {
                GenericArgument::Type(t) => t.clone(),
                _ => return None,
            };
            let value = match iter.next()? {
                GenericArgument::Type(t) => t.clone(),
                _ => return None,
            };
            Some(FieldKind::OrMap { key, value })
        }
        ("OrSet", 1) => {
            let elem = match args.args.first()? {
                GenericArgument::Type(t) => t.clone(),
                _ => return None,
            };
            Some(FieldKind::OrSet { elem })
        }
        _ => None,
    }
}

fn parse_plain_attrs(attrs: &[Attribute]) -> syn::Result<Vec<Meta>> {
    let mut collected = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("plain") {
            continue;
        }
        let metas: Punctuated<Meta, Token![,]> =
            attr.parse_args_with(Punctuated::parse_terminated)?;
        collected.extend(metas);
    }
    Ok(collected)
}

fn parse_default_attr(attrs: &[Attribute]) -> syn::Result<Option<TokenStream>> {
    let mut found: Option<TokenStream> = None;
    for attr in attrs {
        if !attr.path().is_ident("node_attr") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                let lit: LitStr = meta.value()?.parse()?;
                let parsed: TokenStream =
                    lit.value().parse().map_err(|e: proc_macro2::LexError| {
                        syn::Error::new(lit.span(), e.to_string())
                    })?;
                found = Some(parsed);
                Ok(())
            } else {
                Err(meta.error("unknown node_attr key (expected `default`)"))
            }
        })?;
    }
    Ok(found)
}
