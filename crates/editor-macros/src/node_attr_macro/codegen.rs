use proc_macro2::TokenStream;
use quote::quote;

use super::parse::NodeAttrInput;

pub fn generate(input: &NodeAttrInput) -> TokenStream {
    let struct_ident = &input.struct_ident;
    let attr_ident = &input.attr_ident;
    let plain_ident = &input.plain_ident;

    let attr_variants = input.fields.iter().enumerate().map(|(i, s)| {
        let v = &s.variant;
        let t = &s.inner_ty;
        let i_u8 = i as u8;
        quote! { #[wire(n(#i_u8))] #v(#[wire(n(0))] #t) }
    });

    let plain_fields = input.fields.iter().map(|s| {
        let n = &s.name;
        let t = &s.inner_ty;
        let attrs = &s.plain_attrs;
        quote! {
            #( #[#attrs] )*
            pub #n: #t
        }
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

    let to_plain_fields = input.fields.iter().map(|s| {
        let n = &s.name;
        quote! { #n: ::editor_crdt::ToPlain::to_plain(&self.#n) }
    });

    let apply_attr_body = if input.fields.is_empty() {
        quote! { let _ = (id, attr); Ok(()) }
    } else {
        let arms = input.fields.iter().map(|s| {
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
        quote! { match attr { #(#arms),* } }
    };

    let to_attrs_body = if input.fields.is_empty() {
        quote! { ::std::vec::Vec::<#attr_ident>::new() }
    } else {
        let entries = input.fields.iter().map(|s| {
            let n = &s.name;
            let v = &s.variant;
            quote! { #attr_ident::#v(self.#n.clone()) }
        });
        quote! { ::std::vec![#(#entries),*] }
    };

    quote! {
        #[::editor_macros::ffi]
        #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize, ::editor_macros::Wire)]
        #[serde(tag = "type", rename_all = "snake_case")]
        pub enum #attr_ident {
            #(#attr_variants),*
        }

        #[::editor_macros::ffi]
        #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
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
                #apply_attr_body
            }

            pub fn to_plain(&self) -> #plain_ident {
                #plain_ident { #(#to_plain_fields),* }
            }
        }

        impl #plain_ident {
            pub fn to_attrs(&self) -> ::std::vec::Vec<#attr_ident> {
                #to_attrs_body
            }
        }
    }
}
