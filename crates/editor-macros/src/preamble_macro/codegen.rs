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
        ::uniffi::custom_type!(#usize_alias, u64, {
            remote,
            lower: |obj| obj as u64,
            try_lift: |val| Ok(val as usize),
        });
    }
}
