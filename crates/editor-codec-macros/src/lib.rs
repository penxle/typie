mod durable_macro;

use proc_macro::TokenStream;

#[proc_macro_derive(Durable, attributes(durable))]
pub fn derive_durable(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    durable_macro::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
