use syn::parse::ParseStream;
use syn::{DeriveInput, Ident, Type, parenthesized};

pub struct FfiInput {
    pub item: DeriveInput,
    pub custom: Option<Type>,
}

impl FfiInput {
    pub fn from_attr_and_item(attr: proc_macro2::TokenStream, item: DeriveInput) -> Self {
        if attr.is_empty() {
            return Self { item, custom: None };
        }

        let parsed: FfiAttr = syn::parse2(attr).expect("expected `custom` or `custom(TargetType)`");
        Self {
            item,
            custom: Some(parsed.target),
        }
    }
}

struct FfiAttr {
    target: Type,
}

impl syn::parse::Parse for FfiAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        if ident != "custom" {
            return Err(syn::Error::new(ident.span(), "expected `custom`"));
        }

        let content;
        parenthesized!(content in input);
        let target: Type = content.parse()?;

        Ok(FfiAttr { target })
    }
}
