use quote::ToTokens;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Fields, Ident, LitInt, Type};

pub struct WireInput {
    pub ident: Ident,
    pub generics: syn::Generics,
    pub kind: WireKind,
}

pub enum WireKind {
    Enum(WireEnum),
    Struct(WireStruct),
}

pub struct WireEnum {
    pub variants: Vec<WireVariant>,
}

pub struct WireVariant {
    pub ident: Ident,
    pub tag: u8,
    pub fields: Vec<WireField>,
}

pub struct WireStruct {
    pub fields: Vec<WireField>,
    pub transparent: bool,
}

pub struct WireField {
    pub ident: Option<Ident>,
    pub ty: Type,
    pub tag: u8,
    pub skip: bool,
}

impl WireInput {
    pub fn from_derive(input: &DeriveInput) -> syn::Result<Self> {
        let ident = input.ident.clone();
        let generics = input.generics.clone();
        let kind = match &input.data {
            Data::Enum(de) => WireKind::Enum(parse_enum(de)?),
            Data::Struct(ds) => WireKind::Struct(parse_struct(ds, &input.attrs)?),
            Data::Union(_) => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "Wire derive does not support unions",
                ));
            }
        };
        Ok(Self {
            ident,
            generics,
            kind,
        })
    }
}

fn parse_enum(de: &DataEnum) -> syn::Result<WireEnum> {
    let mut variants = Vec::new();
    for v in &de.variants {
        let tag = parse_tag_attr(&v.attrs)?
            .ok_or_else(|| syn::Error::new_spanned(&v.ident, "missing #[wire(n(N))] on variant"))?;
        let fields = parse_fields(&v.fields)?;
        variants.push(WireVariant {
            ident: v.ident.clone(),
            tag,
            fields,
        });
    }
    Ok(WireEnum { variants })
}

fn parse_struct(ds: &DataStruct, attrs: &[syn::Attribute]) -> syn::Result<WireStruct> {
    let transparent = attrs.iter().any(|a| {
        a.path().is_ident("wire") && a.to_token_stream().to_string().contains("transparent")
    });
    let fields = if transparent {
        match &ds.fields {
            Fields::Unnamed(fu) if fu.unnamed.len() == 1 => {
                vec![WireField {
                    ident: None,
                    ty: fu.unnamed[0].ty.clone(),
                    tag: 0,
                    skip: false,
                }]
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    &ds.fields,
                    "#[wire(transparent)] requires a tuple struct with exactly one field",
                ));
            }
        }
    } else {
        parse_fields(&ds.fields)?
    };
    Ok(WireStruct {
        fields,
        transparent,
    })
}

fn parse_fields(fields: &Fields) -> syn::Result<Vec<WireField>> {
    let mut out = Vec::new();
    match fields {
        Fields::Named(fn_) => {
            for f in &fn_.named {
                if has_skip_attr(&f.attrs) {
                    out.push(WireField {
                        ident: f.ident.clone(),
                        ty: f.ty.clone(),
                        tag: 0,
                        skip: true,
                    });
                    continue;
                }
                let tag = parse_tag_attr(&f.attrs)?
                    .ok_or_else(|| syn::Error::new_spanned(f, "missing #[wire(n(N))] on field"))?;
                out.push(WireField {
                    ident: f.ident.clone(),
                    ty: f.ty.clone(),
                    tag,
                    skip: false,
                });
            }
        }
        Fields::Unnamed(fu) => {
            for (i, f) in fu.unnamed.iter().enumerate() {
                if has_skip_attr(&f.attrs) {
                    out.push(WireField {
                        ident: None,
                        ty: f.ty.clone(),
                        tag: 0,
                        skip: true,
                    });
                    continue;
                }
                let tag = parse_tag_attr(&f.attrs)?.unwrap_or(i as u8);
                out.push(WireField {
                    ident: None,
                    ty: f.ty.clone(),
                    tag,
                    skip: false,
                });
            }
        }
        Fields::Unit => {}
    }
    out.sort_by_key(|f| f.tag);
    Ok(out)
}

fn parse_tag_attr(attrs: &[syn::Attribute]) -> syn::Result<Option<u8>> {
    for a in attrs {
        if !a.path().is_ident("wire") {
            continue;
        }
        let mut tag: Option<u8> = None;
        a.parse_nested_meta(|meta| {
            if meta.path.is_ident("n") {
                let content;
                syn::parenthesized!(content in meta.input);
                let lit: LitInt = content.parse()?;
                tag = Some(lit.base10_parse()?);
            }
            Ok(())
        })?;
        if tag.is_some() {
            return Ok(tag);
        }
    }
    Ok(None)
}

fn has_skip_attr(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("wire") && a.to_token_stream().to_string().contains("skip"))
}
