use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;

use super::parse::{EnumVariant, FfiGenInput, StructField};

fn respan(tokens: TokenStream, span: Span) -> TokenStream {
    tokens
        .into_iter()
        .map(|mut tt| {
            match &mut tt {
                TokenTree::Group(g) => {
                    let inner = respan(g.stream(), span);
                    let mut new = proc_macro2::Group::new(g.delimiter(), inner);
                    new.set_span(span);
                    tt = TokenTree::Group(new);
                }
                _ => tt.set_span(span),
            }
            tt
        })
        .collect()
}

fn respan_type(ty: &syn::Type) -> TokenStream {
    respan(quote! { #ty }, Span::call_site())
}

pub fn generate(input: &FfiGenInput) -> TokenStream {
    match input {
        FfiGenInput::CustomType { source, target } => gen_custom_type(source, target),
        FfiGenInput::Struct {
            name,
            source,
            fields,
        } => gen_struct(name, source, fields),
        FfiGenInput::Enum {
            name,
            source,
            variants,
        } => gen_enum(name, source, variants),
    }
}

fn gen_identity_impls(source: &syn::Path) -> TokenStream {
    quote! {
        impl crate::convert::FromFfi<#source> for #source {
            fn from_ffi(self) -> Result<#source, crate::error::FfiError> { Ok(self) }
        }

        impl crate::convert::IntoFfi<#source> for #source {
            fn into_ffi(self) -> Result<#source, crate::error::FfiError> { Ok(self) }
        }
    }
}

fn gen_wasm_impls(name: &syn::Ident, source: &syn::Path) -> TokenStream {
    quote! {
        #[cfg(feature = "wasm")]
        impl crate::convert::FromFfi<#source> for wasm_bindgen::JsValue {
            fn from_ffi(self) -> Result<#source, crate::error::FfiError> {
                let ffi: #name = serde_wasm_bindgen::from_value(self)
                    .map_err(|e| crate::error::FfiError::Deserialization(
                        format!(concat!(stringify!(#name), ": {:?}"), e)
                    ))?;
                crate::convert::FromFfi::from_ffi(ffi)
            }
        }

        #[cfg(feature = "wasm")]
        impl crate::convert::IntoFfi<wasm_bindgen::JsValue> for #source {
            fn into_ffi(self) -> Result<wasm_bindgen::JsValue, crate::error::FfiError> {
                let ffi: #name = crate::convert::IntoFfi::into_ffi(self)?;
                serde_wasm_bindgen::to_value(&ffi)
                    .map_err(|e| crate::error::FfiError::Serialization(
                        format!(concat!(stringify!(#name), ": {:?}"), e)
                    ))
            }
        }
    }
}

fn gen_custom_type(source: &syn::Path, target: &syn::Type) -> TokenStream {
    let type_name = &source.segments.last().unwrap().ident;
    let identity = gen_identity_impls(source);
    quote! {
        use #source;

        #[cfg(feature = "uniffi")]
        ::uniffi::custom_type!(#type_name, <#source as ::editor_common::Ffi>::Target, {
            remote,
            lower: |obj| ::editor_common::Ffi::to_ffi(&obj),
            try_lift: |val| Ok(::editor_common::Ffi::from_ffi(val)?),
        });

        #[cfg(feature = "wasm")]
        const _: () = {
            #[derive(::serde::Serialize, ::serde::Deserialize, ::tsify::Tsify)]
            #[serde(transparent)]
            struct #type_name(#target);
        };

        #identity
    }
}

fn gen_struct(name: &syn::Ident, source: &syn::Path, fields: &[StructField]) -> TokenStream {
    let field_defs: Vec<_> = fields
        .iter()
        .map(|f| {
            let fname = &f.name;
            let fty = respan_type(&f.ty);
            quote! { pub #fname: #fty }
        })
        .collect();

    let from_ffi_fields: Vec<_> = fields
        .iter()
        .map(|f| {
            let fname = &f.name;
            quote! { #fname: crate::convert::FromFfi::from_ffi(self.#fname)? }
        })
        .collect();

    let into_ffi_fields: Vec<_> = fields
        .iter()
        .map(|f| {
            let fname = &f.name;
            quote! { #fname: crate::convert::IntoFfi::into_ffi(self.#fname)? }
        })
        .collect();

    let identity = gen_identity_impls(source);
    let wasm = gen_wasm_impls(name, source);

    quote! {
        #[cfg_attr(feature = "uniffi", derive(::uniffi::Record))]
        #[cfg_attr(feature = "wasm", derive(::serde::Serialize, ::serde::Deserialize, ::tsify::Tsify))]
        #[derive(Debug, Clone)]
        pub struct #name {
            #(#field_defs,)*
        }

        impl crate::convert::FromFfi<#source> for #name {
            fn from_ffi(self) -> Result<#source, crate::error::FfiError> {
                Ok(#source { #(#from_ffi_fields,)* })
            }
        }

        impl crate::convert::IntoFfi<#name> for #source {
            fn into_ffi(self) -> Result<#name, crate::error::FfiError> {
                Ok(#name { #(#into_ffi_fields,)* })
            }
        }

        #wasm
        #identity
    }
}

fn gen_enum(name: &syn::Ident, source: &syn::Path, variants: &[EnumVariant]) -> TokenStream {
    let mut variant_defs = Vec::new();
    let mut from_ffi_arms = Vec::new();
    let mut into_ffi_arms = Vec::new();

    for v in variants {
        match v {
            EnumVariant::Unit {
                name: vname,
                source_path,
            } => {
                variant_defs.push(quote! { #vname });
                from_ffi_arms.push(quote! { #name::#vname => Ok(#source_path) });
                into_ffi_arms.push(quote! { #source_path => Ok(#name::#vname) });
            }
            EnumVariant::Tuple {
                name: vname,
                bindings,
                source_path,
            } => {
                let types: Vec<_> = bindings.iter().map(|(_, ty)| respan_type(ty)).collect();
                let vars: Vec<_> = bindings.iter().map(|(var, _)| var).collect();
                variant_defs.push(quote! { #vname(#(#types),*) });
                from_ffi_arms.push(quote! {
                    #name::#vname(#(#vars),*) => Ok(#source_path(#(crate::convert::FromFfi::from_ffi(#vars)?),*))
                });
                into_ffi_arms.push(quote! {
                    #source_path(#(#vars),*) => Ok(#name::#vname(#(crate::convert::IntoFfi::into_ffi(#vars)?),*))
                });
            }
            EnumVariant::Named {
                name: vname,
                fields,
                source_path,
            } => {
                let field_names: Vec<_> = fields.iter().map(|f| &f.name).collect();
                let field_types: Vec<_> = fields.iter().map(|f| respan_type(&f.ty)).collect();
                variant_defs.push(quote! { #vname { #(#field_names: #field_types),* } });
                from_ffi_arms.push(quote! {
                    #name::#vname { #(#field_names),* } =>
                        Ok(#source_path { #(#field_names: crate::convert::FromFfi::from_ffi(#field_names)?),* })
                });
                into_ffi_arms.push(quote! {
                    #source_path { #(#field_names),* } =>
                        Ok(#name::#vname { #(#field_names: crate::convert::IntoFfi::into_ffi(#field_names)?),* })
                });
            }
        }
    }

    let identity = gen_identity_impls(source);
    let wasm = gen_wasm_impls(name, source);

    quote! {
        #[cfg_attr(feature = "uniffi", derive(::uniffi::Enum))]
        #[cfg_attr(feature = "wasm", derive(::serde::Serialize, ::serde::Deserialize, ::tsify::Tsify))]
        #[derive(Debug, Clone)]
        pub enum #name {
            #(#variant_defs,)*
        }

        impl crate::convert::FromFfi<#source> for #name {
            fn from_ffi(self) -> Result<#source, crate::error::FfiError> {
                match self {
                    #(#from_ffi_arms,)*
                }
            }
        }

        impl crate::convert::IntoFfi<#name> for #source {
            fn into_ffi(self) -> Result<#name, crate::error::FfiError> {
                match self {
                    #(#into_ffi_arms,)*
                }
            }
        }

        #wasm
        #identity
    }
}

pub fn generate_derive(path: &syn::Path) -> TokenStream {
    let segments: Vec<_> = path.segments.iter().collect();

    if segments.len() != 2 {
        panic!("derive_ffi! expects `crate_name::TypeName`");
    }

    let crate_name = &segments[0].ident;
    let type_name = &segments[1].ident;
    let describe_macro = quote::format_ident!("__ffi_describe_{}", type_name);

    quote! {
        #crate_name :: #describe_macro ! (editor_macros::__ffi_gen);
    }
}
