use syn::{Data, DataEnum, DeriveInput, Field, Fields, Variant};

pub struct ModifierStateInput {
    pub enum_ident: syn::Ident,
    pub variants: Vec<VariantInfo>,
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
        Ok(Self {
            enum_ident,
            variants,
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
