use proc_macro2::TokenStream;
use quote::quote;

use super::parse::FfiInput;

pub fn generate(input: &FfiInput) -> TokenStream {
    let item = &input.item;

    if let Some(custom) = &input.custom {
        let ident = item.ident.clone();

        quote! {
            #item

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
        }
    }
}
