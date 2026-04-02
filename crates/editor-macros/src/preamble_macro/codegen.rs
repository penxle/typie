use heck::ToSnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub fn generate() -> TokenStream {
    let crate_name = std::env::var("CARGO_PKG_NAME")
        .expect("CARGO_PKG_NAME not set")
        .to_snake_case();
    let crate_ident = Ident::new(&crate_name, Span::call_site());

    quote! {
        #[cfg(not(doctest))]
        extern crate self as #crate_ident;

        #[cfg(feature = "uniffi")]
        ::uniffi::setup_scaffolding!();

        #[cfg(feature = "uniffi")]
        ::uniffi::custom_type!(usize, u64, {
            remote,
            lower: |obj| obj as u64,
            try_lift: |val| Ok(val as usize),
        });
    }
}
