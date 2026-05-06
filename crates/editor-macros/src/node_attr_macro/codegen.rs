use proc_macro2::TokenStream;
use quote::quote;

use super::parse::NodeAttrInput;

pub fn generate(input: &NodeAttrInput) -> TokenStream {
    let struct_ident = &input.struct_ident;
    let attr_ident = &input.attr_ident;
    let plain_ident = &input.plain_ident;

    let attr_variants = input.fields.iter().map(|s| {
        let v = &s.variant;
        let t = &s.inner_ty;
        quote! { #v(#t) }
    });

    let plain_fields = input.fields.iter().map(|s| {
        let n = &s.name;
        let t = &s.inner_ty;
        quote! { pub #n: #t }
    });

    let struct_default_assigns = input.fields.iter().map(|s| {
        let n = &s.name;
        match &s.default {
            Some(expr) => quote! { #n: ::editor_crdt::LwwReg::with_value(#expr) },
            None => quote! { #n: ::editor_crdt::LwwReg::default() },
        }
    });

    let plain_default_assigns = input.fields.iter().map(|s| {
        let n = &s.name;
        let t = &s.inner_ty;
        match &s.default {
            Some(expr) => quote! { #n: #expr },
            None => quote! { #n: <#t as ::std::default::Default>::default() },
        }
    });

    let apply_arms = input.fields.iter().map(|s| {
        let n = &s.name;
        let v = &s.variant;
        quote! {
            #attr_ident::#v(value) => {
                self.#n = self.#n.apply(
                    id,
                    ::editor_crdt::LwwRegOp::Set { value: value.clone() },
                )?;
                Ok(())
            }
        }
    });

    let to_plain_fields = input.fields.iter().map(|s| {
        let n = &s.name;
        quote! { #n: ::editor_crdt::ToPlain::to_plain(&self.#n) }
    });

    quote! {
        #[derive(Debug, Clone, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
        pub enum #attr_ident {
            #(#attr_variants),*
        }

        #[derive(Debug, Clone, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
        pub struct #plain_ident {
            #(#plain_fields),*
        }

        impl ::std::default::Default for #struct_ident {
            fn default() -> Self {
                Self { #(#struct_default_assigns),* }
            }
        }

        impl ::std::default::Default for #plain_ident {
            fn default() -> Self {
                Self { #(#plain_default_assigns),* }
            }
        }

        impl #struct_ident {
            pub fn apply_attr(
                &mut self,
                id: ::editor_crdt::Dot,
                attr: &#attr_ident,
            ) -> ::std::result::Result<(), ::editor_crdt::CrdtError> {
                match attr {
                    #(#apply_arms),*
                }
            }

            pub fn to_plain(&self) -> #plain_ident {
                #plain_ident { #(#to_plain_fields),* }
            }
        }
    }
}
