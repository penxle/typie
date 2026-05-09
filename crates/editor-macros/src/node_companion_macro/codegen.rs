use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::NodeCompanionInput;

pub fn generate(input: &NodeCompanionInput) -> TokenStream {
    let enum_ident = &input.enum_ident;

    let plain_node_variants = input.variants.iter().map(|v| {
        let n = &v.variant_ident;
        let plain_inner = format_ident!("Plain{}", v.inner_type_ident);
        quote! { #n(#plain_inner) }
    });

    let to_plain_arms = input.variants.iter().map(|v| {
        let n = &v.variant_ident;
        quote! { #enum_ident::#n(inner) => PlainNode::#n(inner.to_plain()) }
    });

    let as_type_arms = input.variants.iter().map(|v| {
        let n = &v.variant_ident;
        quote! { PlainNode::#n(_) => NodeType::#n }
    });

    let to_attrs_arms = input.variants.iter().map(|v| {
        let n = &v.variant_ident;
        quote! {
            PlainNode::#n(inner) => inner
                .to_attrs()
                .into_iter()
                .map(|attr| NodeAttr::#n { attr })
                .collect()
        }
    });

    let node_attr_variants = input.variants.iter().enumerate().map(|(i, v)| {
        let n = &v.variant_ident;
        let attr_ty = format_ident!("{}Attr", v.inner_type_ident);
        let i_lit = i as u32;
        quote! { #[n(#i_lit)] #n { #[n(0)] attr: #attr_ty } }
    });

    let apply_attr_arms = input.variants.iter().map(|v| {
        let n = &v.variant_ident;
        quote! {
            (#enum_ident::#n(inner), NodeAttr::#n { attr }) => {
                inner.apply_attr(id, attr).map_err(ModelError::from)
            }
        }
    });

    quote! {
        #[::editor_macros::ffi]
        #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        pub enum PlainNode {
            #(#plain_node_variants),*
        }

        #[::editor_macros::ffi]
        #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize, ::minicbor::Encode, ::minicbor::Decode)]
        #[serde(tag = "type", rename_all = "snake_case")]
        pub enum NodeAttr {
            #(#node_attr_variants),*
        }

        impl #enum_ident {
            pub fn to_plain(&self) -> PlainNode {
                match self {
                    #(#to_plain_arms),*
                }
            }

            pub fn apply_attr(
                &mut self,
                id: ::editor_crdt::Dot,
                attr: &NodeAttr,
            ) -> ::std::result::Result<(), ModelError> {
                match (self, attr) {
                    #(#apply_attr_arms),*,
                    _ => Err(ModelError::AttrNodeKindMismatch),
                }
            }
        }

        impl PlainNode {
            pub fn as_type(&self) -> NodeType {
                match self {
                    #(#as_type_arms),*
                }
            }

            pub fn to_attrs(&self) -> ::std::vec::Vec<NodeAttr> {
                match self {
                    #(#to_attrs_arms),*
                }
            }
        }
    }
}
