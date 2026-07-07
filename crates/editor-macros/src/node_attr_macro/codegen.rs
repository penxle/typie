use proc_macro2::TokenStream;
use quote::quote;

use super::parse::{FieldKind, NodeAttrInput};

pub fn generate(input: &NodeAttrInput) -> TokenStream {
    let struct_ident = &input.struct_ident;
    let attr_ident = &input.attr_ident;
    let plain_ident = &input.plain_ident;

    let attr_variants = input.fields.iter().map(|s| {
        let v = &s.variant;
        let payload = match &s.kind {
            FieldKind::LwwReg { inner } => quote! { #inner },
            FieldKind::OrMap { key, value } => {
                quote! { ::editor_crdt::OrMapOp<#key, #value> }
            }
            FieldKind::OrSet { elem } => quote! { ::editor_crdt::OrSetOp<#elem> },
        };
        quote! { #v(#payload) }
    });

    let plain_fields = input.fields.iter().map(|s| {
        let n = &s.name;
        let attrs = &s.plain_attrs;
        let plain_ty = match &s.kind {
            FieldKind::LwwReg { inner } => quote! { #inner },
            FieldKind::OrMap { key, value } => {
                quote! { ::std::collections::BTreeMap<#key, #value> }
            }
            FieldKind::OrSet { elem } => quote! { ::std::collections::BTreeSet<#elem> },
        };
        quote! {
            #( #[#attrs] )*
            pub #n: #plain_ty
        }
    });

    let struct_default_assigns = input.fields.iter().map(|s| {
        let n = &s.name;
        match &s.kind {
            FieldKind::LwwReg { .. } => match &s.default {
                Some(expr) => quote! { #n: ::editor_crdt::LwwReg::with_value(#expr) },
                None => quote! { #n: ::editor_crdt::LwwReg::default() },
            },
            FieldKind::OrMap { .. } => quote! { #n: ::editor_crdt::OrMap::default() },
            FieldKind::OrSet { .. } => quote! { #n: ::editor_crdt::OrSet::default() },
        }
    });

    let plain_default_assigns = input.fields.iter().map(|s| {
        let n = &s.name;
        match &s.kind {
            FieldKind::LwwReg { inner } => match &s.default {
                Some(expr) => quote! { #n: #expr },
                None => quote! { #n: <#inner as ::std::default::Default>::default() },
            },
            FieldKind::OrMap { key, value } => {
                quote! {
                    #n: <::std::collections::BTreeMap<#key, #value>
                        as ::std::default::Default>::default()
                }
            }
            FieldKind::OrSet { elem } => {
                quote! {
                    #n: <::std::collections::BTreeSet<#elem>
                        as ::std::default::Default>::default()
                }
            }
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
            match &s.kind {
                FieldKind::LwwReg { .. } => quote! {
                    #attr_ident::#v(value) => {
                        self.#n = self.#n.apply(
                            id,
                            ::editor_crdt::LwwRegOp::Set { value: value.clone() },
                        )?;
                        Ok(())
                    }
                },
                FieldKind::OrMap { .. } | FieldKind::OrSet { .. } => quote! {
                    #attr_ident::#v(op) => {
                        self.#n = self.#n.apply(id, op.clone())?;
                        Ok(())
                    }
                },
            }
        });
        quote! { match attr { #(#arms),* } }
    };

    let to_attrs_body = if input.fields.is_empty() {
        quote! { ::std::vec::Vec::<#attr_ident>::new() }
    } else {
        let pushes = input.fields.iter().map(|s| {
            let n = &s.name;
            let v = &s.variant;
            match &s.kind {
                FieldKind::LwwReg { .. } => quote! {
                    out.push(#attr_ident::#v(self.#n.clone()));
                },
                FieldKind::OrMap { .. } => quote! {
                    for (key, value) in self.#n.iter() {
                        out.push(#attr_ident::#v(::editor_crdt::OrMapOp::Set {
                            key: key.clone(),
                            value: value.clone(),
                        }));
                    }
                },
                FieldKind::OrSet { .. } => quote! {
                    for elem in self.#n.iter() {
                        out.push(#attr_ident::#v(::editor_crdt::OrSetOp::Add {
                            elem: elem.clone(),
                        }));
                    }
                },
            }
        });
        quote! {
            {
                let mut out = ::std::vec::Vec::<#attr_ident>::new();
                #(#pushes)*
                out
            }
        }
    };

    quote! {
        #[::editor_macros::ffi]
        #[derive(Debug, Clone, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize)]
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
