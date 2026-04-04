use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::meta;
use super::parse::FfiInput;

fn strip_ffi_attrs(mut item: syn::DeriveInput) -> syn::DeriveInput {
    match &mut item.data {
        syn::Data::Struct(data) => {
            for field in &mut data.fields {
                field.attrs.retain(|attr| !attr.path().is_ident("ffi"));
            }
        }
        syn::Data::Enum(data) => {
            for variant in &mut data.variants {
                for field in &mut variant.fields {
                    field.attrs.retain(|attr| !attr.path().is_ident("ffi"));
                }
            }
        }
        syn::Data::Union(_) => {}
    }
    item
}

pub fn generate(input: &FfiInput) -> TokenStream {
    let item = strip_ffi_attrs(input.item.clone());
    let meta_static = generate_meta_static(input);

    if let Some(custom) = &input.custom {
        let ident = item.ident.clone();

        quote! {
            #item

            #meta_static

            #[cfg(feature = "wasm")]
            const _: () = {
                #[derive(::tsify::Tsify)]
                #[tsify(hashmap_as_object)]
                struct #ident(#custom);
            };

            #[cfg(feature = "uniffi")]
            ::uniffi::custom_type!(#ident, #custom, {
                lower: |obj| ::editor_common::Ffi::to_ffi(&obj),
                try_lift: |val| ::editor_common::Ffi::from_ffi(val).map_err(Into::into),
            });
        }
    } else {
        let uniffi_derive = match &item.data {
            syn::Data::Struct(_) => {
                quote! { #[cfg_attr(feature = "uniffi", derive(::uniffi::Record))] }
            }
            syn::Data::Enum(_) => {
                quote! { #[cfg_attr(feature = "uniffi", derive(::uniffi::Enum))] }
            }
            syn::Data::Union(_) => panic!("#[ffi] does not support unions"),
        };

        quote! {
            #uniffi_derive
            #[cfg_attr(feature = "wasm", derive(::tsify::Tsify))]
            #[cfg_attr(feature = "wasm", tsify(hashmap_as_object))]
            #item

            #meta_static
        }
    }
}

pub fn generate_type_alias(item: &syn::ItemType) -> TokenStream {
    let name = item.ident.to_string();
    let target = meta::type_to_string(&item.ty);

    let ffi_meta = editor_bindgen::meta::FfiMeta {
        name,
        serde_rename_all: None,
        kind: editor_bindgen::meta::FfiKind::Custom { target },
    };

    let encoded = bitcode::encode(&ffi_meta);
    let payload_len = encoded.len() as u32;
    let total_len = 4 + encoded.len();
    let prefix = payload_len.to_le_bytes();
    let all_bytes: Vec<u8> = prefix.iter().copied().chain(encoded).collect();

    let crate_name = std::env::var("CARGO_PKG_NAME")
        .unwrap_or_default()
        .replace('-', "_");
    let ident = format_ident!("FFI_META_{}_{}", crate_name, item.ident);

    quote! {
        #item

        #[used]
        #[unsafe(no_mangle)]
        pub static #ident: [u8; #total_len] = [#(#all_bytes),*];
    }
}

fn generate_meta_static(input: &FfiInput) -> TokenStream {
    let meta = meta::extract(&input.item, input.custom.as_ref());
    let encoded = bitcode::encode(&meta);

    let payload_len = encoded.len() as u32;
    let total_len = 4 + encoded.len();

    let prefix = payload_len.to_le_bytes();
    let all_bytes: Vec<u8> = prefix.iter().copied().chain(encoded).collect();

    let crate_name = std::env::var("CARGO_PKG_NAME")
        .unwrap_or_default()
        .replace('-', "_");
    let ident = format_ident!("FFI_META_{}_{}", crate_name, input.item.ident);

    quote! {
        #[unsafe(no_mangle)]
        pub static #ident: [u8; #total_len] = [#(#all_bytes),*];
    }
}
