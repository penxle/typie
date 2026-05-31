use syn::{Data, DataEnum, DeriveInput, Field, Fields, Variant};

pub struct ModifierStateInput {
    pub enum_ident: syn::Ident,
    pub variants: Vec<VariantInfo>,
    pub computed: Vec<syn::Ident>,
}

pub enum VariantInfo {
    Unit {
        ident: syn::Ident,
    },
    StructLike {
        ident: syn::Ident,
        fields: Vec<Field>,
    },
}

impl ModifierStateInput {
    pub fn from_derive(input: &DeriveInput) -> syn::Result<Self> {
        let enum_ident = input.ident.clone();
        let Data::Enum(DataEnum { variants, .. }) = &input.data else {
            return Err(syn::Error::new_spanned(
                input,
                "ModifierState can only be derived on enums",
            ));
        };
        let variants = variants
            .iter()
            .map(VariantInfo::from_variant)
            .collect::<syn::Result<Vec<_>>>()?;

        let mut computed = Vec::new();
        for attr in &input.attrs {
            if !attr.path().is_ident("modifier_state") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("computed") {
                    meta.parse_nested_meta(|inner| {
                        computed.push(inner.path.require_ident()?.clone());
                        Ok(())
                    })
                } else {
                    Err(meta.error("unknown `modifier_state` option"))
                }
            })?;
        }

        Ok(Self {
            enum_ident,
            variants,
            computed,
        })
    }
}

impl VariantInfo {
    fn from_variant(v: &Variant) -> syn::Result<Self> {
        match &v.fields {
            Fields::Unit => Ok(VariantInfo::Unit {
                ident: v.ident.clone(),
            }),
            Fields::Named(named) => Ok(VariantInfo::StructLike {
                ident: v.ident.clone(),
                fields: named.named.iter().cloned().collect(),
            }),
            Fields::Unnamed(_) => Err(syn::Error::new_spanned(
                v,
                "ModifierState requires struct-like (named-field) variants only; tuple variants are not supported",
            )),
        }
    }
}
