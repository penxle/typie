use heck::ToSnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub fn generate() -> TokenStream {
    let crate_name = std::env::var("CARGO_PKG_NAME")
        .expect("CARGO_PKG_NAME not set")
        .to_snake_case();
    let crate_ident = Ident::new(&crate_name, Span::call_site());
    let usize_alias = Ident::new(&format!("__{crate_name}_usize"), Span::call_site());

    quote! {
        #[cfg(not(doctest))]
        extern crate self as #crate_ident;

        #[cfg(feature = "uniffi")]
        ::uniffi::setup_scaffolding!();

        #[cfg(feature = "uniffi")]
        type #usize_alias = usize;

        #[cfg(feature = "uniffi")]
        ::uniffi::custom_type!(#usize_alias, u32, {
            remote,
            // Editor document sizes stay below u32::MAX; release-build truncation is intentional.
            lower: |obj| {
                debug_assert!(
                    obj <= u32::MAX as usize,
                    "usize value {} exceeds u32::MAX at FFI boundary",
                    obj
                );
                obj as u32
            },
            try_lift: |val| Ok(val as usize),
        });
    }
}
