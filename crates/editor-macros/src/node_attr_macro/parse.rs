use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{
    Attribute, Data, DeriveInput, Fields, GenericArgument, LitStr, Meta, PathArguments, Token,
    Type, punctuated::Punctuated,
};

pub struct FieldSpec {
    pub name: syn::Ident,
    pub variant: syn::Ident,
    pub inner_ty: Type,
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
            let inner = match lwwreg_inner(&f.ty) {
                Some(i) => i.clone(),
                None => {
                    return Err(syn::Error::new_spanned(
                        f,
                        format!("NodeAttr field `{}` must be `LwwReg<T>`", name),
                    ));
                }
            };
            let variant = format_ident!("{}", heck::AsPascalCase(name.to_string()).to_string());
            let default = parse_default_attr(&f.attrs)?;
            let plain_attrs = parse_plain_attrs(&f.attrs)?;
            fields.push(FieldSpec {
                name,
                variant,
                inner_ty: inner,
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

fn lwwreg_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(tp) = ty else { return None };
    let last = tp.path.segments.last()?;
    if last.ident != "LwwReg" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    if args.args.len() != 1 {
        return None;
    }
    match args.args.first()? {
        GenericArgument::Type(t) => Some(t),
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
