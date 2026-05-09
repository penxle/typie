use syn::{Data, DeriveInput, Fields, Type, TypePath};

pub struct NodeCompanionInput {
    pub enum_ident: syn::Ident,
    pub variants: Vec<NodeVariant>,
}

pub struct NodeVariant {
    pub variant_ident: syn::Ident,
    pub inner_type_ident: syn::Ident,
}

impl NodeCompanionInput {
    pub fn from_derive(derive: &DeriveInput) -> syn::Result<Self> {
        let Data::Enum(data) = &derive.data else {
            return Err(syn::Error::new_spanned(
                derive,
                "NodeCompanion requires an enum",
            ));
        };

        let variants: Vec<NodeVariant> = data
            .variants
            .iter()
            .map(|v| {
                let Fields::Unnamed(unnamed) = &v.fields else {
                    return Err(syn::Error::new_spanned(
                        v,
                        "NodeCompanion requires tuple-struct variants",
                    ));
                };
                if unnamed.unnamed.len() != 1 {
                    return Err(syn::Error::new_spanned(
                        v,
                        "NodeCompanion requires single-field tuple variants",
                    ));
                }
                let Type::Path(TypePath { path, .. }) = &unnamed.unnamed[0].ty else {
                    return Err(syn::Error::new_spanned(
                        &unnamed.unnamed[0],
                        "expected path type",
                    ));
                };
                let inner_type_ident = path.segments.last().unwrap().ident.clone();
                Ok(NodeVariant {
                    variant_ident: v.ident.clone(),
                    inner_type_ident,
                })
            })
            .collect::<syn::Result<Vec<_>>>()?;

        Ok(Self {
            enum_ident: derive.ident.clone(),
            variants,
        })
    }
}
