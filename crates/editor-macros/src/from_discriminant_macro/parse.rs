use syn::parse::{Parse, ParseStream};
use syn::{Data, DeriveInput, Fields, Ident, Type};

pub struct FromDiscriminantInput {
    pub enum_name: Ident,
    pub discriminant_type: Ident,
    pub variants: Vec<VariantInfo>,
}

pub struct VariantInfo {
    pub name: Ident,
    pub inner_type: Type,
}

impl Parse for FromDiscriminantInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let derive_input: DeriveInput = input.parse()?;

        let enum_name = derive_input.ident.clone();

        let discriminant_type = parse_discriminant_attr(&derive_input)
            .ok_or_else(|| input.error("Expected #[from_discriminant(TypeName)] attribute"))?;

        let variants = match &derive_input.data {
            Data::Enum(data) => data
                .variants
                .iter()
                .map(|v| {
                    let inner_type = match &v.fields {
                        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                            fields.unnamed[0].ty.clone()
                        }
                        _ => {
                            return Err(syn::Error::new_spanned(
                                v,
                                "FromDiscriminant requires all variants to be newtype (single unnamed field)",
                            ))
                        }
                    };
                    Ok(VariantInfo {
                        name: v.ident.clone(),
                        inner_type,
                    })
                })
                .collect::<syn::Result<Vec<_>>>()?,
            _ => return Err(input.error("FromDiscriminant can only be derived for enums")),
        };

        Ok(FromDiscriminantInput {
            enum_name,
            discriminant_type,
            variants,
        })
    }
}

fn parse_discriminant_attr(input: &DeriveInput) -> Option<Ident> {
    for attr in &input.attrs {
        if attr.path().is_ident("from_discriminant") {
            let ident: Ident = attr.parse_args().ok()?;
            return Some(ident);
        }
    }
    None
}
