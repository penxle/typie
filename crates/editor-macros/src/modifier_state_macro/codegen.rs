use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::{ModifierStateInput, VariantInfo};

pub fn generate(input: &ModifierStateInput) -> TokenStream {
    let wrapper_structs = wrapper_structs(input);
    let state_struct = state_struct(input);

    quote! {
        #wrapper_structs
        #state_struct
    }
}

fn wrapper_structs(input: &ModifierStateInput) -> TokenStream {
    let mut out = TokenStream::new();
    for v in &input.variants {
        if let VariantInfo::StructLike { ident, fields } = v {
            let wrapper = wrapper_ident(ident);
            let field_decls = fields.iter().map(|f| {
                let name = &f.ident;
                let ty = &f.ty;
                quote! { pub #name: #ty }
            });
            out.extend(quote! {
                #[::editor_macros::ffi]
                #[derive(Clone, Debug, PartialEq, Eq, ::std::hash::Hash, ::serde::Serialize, ::serde::Deserialize)]
                pub struct #wrapper {
                    #(#field_decls,)*
                }
            });
        }
    }
    out
}

fn state_struct(input: &ModifierStateInput) -> TokenStream {
    let state_ident = format_ident!("{}State", input.enum_ident);
    let fields = input.variants.iter().map(|v| {
        let (ident, ty_inner) = match v {
            VariantInfo::Unit { ident } => (ident.clone(), quote! { () }),
            VariantInfo::StructLike { ident, .. } => {
                let wrapper = wrapper_ident(ident);
                (ident.clone(), quote! { #wrapper })
            }
        };
        let field_name = format_ident!("{}", ident.to_string().to_snake_case());
        quote! { pub #field_name: ::editor_common::Tri<#ty_inner> }
    });

    let computed_fields = input.computed.iter().map(|ident| {
        quote! { pub #ident: ::editor_common::Tri<()> }
    });

    quote! {
        #[::editor_macros::ffi]
        #[derive(Clone, Debug, PartialEq, Eq, ::std::hash::Hash, Default, ::serde::Serialize, ::serde::Deserialize)]
        pub struct #state_ident {
            #(#fields,)*
            #(#computed_fields,)*
        }
    }
}

fn wrapper_ident(variant: &syn::Ident) -> syn::Ident {
    format_ident!("{}Value", variant)
}
