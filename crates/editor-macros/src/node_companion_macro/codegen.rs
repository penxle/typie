use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::NodeCompanionInput;

pub fn generate(input: &NodeCompanionInput) -> TokenStream {
    let enum_ident = &input.enum_ident;

    // `Unknown` is an attr-less placeholder node — it has no `{Variant}Attr`
    // companion type, so it is excluded from every per-variant list below and
    // wired by hand instead (its `NodeAttr::Unknown{tag,bytes}` shape is
    // injected separately and would collide with the generic `{attr}` shape).
    let known: Vec<_> = input
        .variants
        .iter()
        .filter(|v| v.variant_ident != "Unknown")
        .collect();

    let plain_node_variants = known.iter().map(|v| {
        let n = &v.variant_ident;
        let plain_inner = format_ident!("Plain{}", v.inner_type_ident);
        quote! { #n(#plain_inner) }
    });

    let to_plain_arms = known.iter().map(|v| {
        let n = &v.variant_ident;
        quote! { #enum_ident::#n(inner) => PlainNode::#n(inner.to_plain()) }
    });

    let as_type_arms = known.iter().map(|v| {
        let n = &v.variant_ident;
        quote! { PlainNode::#n(_) => NodeType::#n }
    });

    let to_attrs_arms = known.iter().map(|v| {
        let n = &v.variant_ident;
        quote! {
            PlainNode::#n(inner) => inner
                .to_attrs()
                .into_iter()
                .map(|attr| NodeAttr::#n { attr })
                .collect()
        }
    });

    let node_attr_variants = known.iter().map(|v| {
        let n = &v.variant_ident;
        let attr_ty = format_ident!("{}Attr", v.inner_type_ident);
        quote! { #n { attr: #attr_ty } }
    });

    let apply_attr_arms = known.iter().map(|v| {
        let n = &v.variant_ident;
        quote! {
            (#enum_ident::#n(inner), NodeAttr::#n { attr }) => {
                inner.apply_attr(id, attr).map_err(ModelError::from)
            }
        }
    });

    let same_field_arms = known.iter().map(|v| {
        let n = &v.variant_ident;
        quote! {
            (NodeAttr::#n { attr: a }, NodeAttr::#n { attr: b }) =>
                ::std::mem::discriminant(a) == ::std::mem::discriminant(b)
        }
    });

    quote! {
        #[::editor_macros::ffi]
        #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        pub enum PlainNode {
            #(#plain_node_variants),*,
            Unknown,
        }

        #[::editor_macros::ffi]
        #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        pub enum NodeAttr {
            #(#node_attr_variants),*,
            Unknown {
                tag: u64,
                bytes: Vec<u8>,
            },
        }

        impl NodeAttr {
            pub fn same_field(&self, other: &Self) -> bool {
                match (self, other) {
                    #(#same_field_arms,)*
                    (NodeAttr::Unknown { tag: a, .. }, NodeAttr::Unknown { tag: b, .. }) => a == b,
                    _ => false,
                }
            }
        }

        impl #enum_ident {
            pub fn to_plain(&self) -> PlainNode {
                match self {
                    #(#to_plain_arms),*,
                    #enum_ident::Unknown(_) => PlainNode::Unknown,
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
                    #(#as_type_arms),*,
                    PlainNode::Unknown => NodeType::Unknown,
                }
            }

            pub fn to_attrs(&self) -> ::std::vec::Vec<NodeAttr> {
                match self {
                    #(#to_attrs_arms),*,
                    PlainNode::Unknown => ::std::vec::Vec::new(),
                }
            }
        }
    }
}
