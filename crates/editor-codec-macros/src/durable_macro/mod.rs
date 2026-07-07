pub mod codegen;
pub mod parse;
pub mod schema_gen;

use proc_macro2::TokenStream;

pub fn expand(input: &syn::DeriveInput) -> syn::Result<TokenStream> {
    let parsed = parse::parse(input)?;
    let mut out = codegen::generate(&parsed);
    out.extend(schema_gen::generate(&parsed));
    Ok(out)
}
