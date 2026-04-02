mod content_macro;
mod context_macro;
mod doc_macro;
mod ffi_macro;
mod from_discriminant_macro;
mod preamble_macro;
mod state_macro;

use proc_macro::TokenStream;

#[proc_macro]
pub fn preamble(_input: TokenStream) -> TokenStream {
    preamble_macro::codegen::generate().into()
}

#[proc_macro]
pub fn content_expr(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as content_macro::parse::ContentAst);
    content_macro::codegen::generate(&ast).into()
}

#[proc_macro]
pub fn context_expr(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as context_macro::parse::ContextAst);
    context_macro::codegen::generate(&ast).into()
}

#[proc_macro]
pub fn doc(input: TokenStream) -> TokenStream {
    let tree = syn::parse_macro_input!(input as doc_macro::parse::DocTree);
    doc_macro::codegen::generate(&tree).into()
}

#[proc_macro]
pub fn state(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as state_macro::parse::StateInput);
    state_macro::codegen::generate(&input).into()
}

#[proc_macro_derive(FromDiscriminant, attributes(from_discriminant))]
pub fn derive_from_discriminant(input: TokenStream) -> TokenStream {
    let input =
        syn::parse_macro_input!(input as from_discriminant_macro::parse::FromDiscriminantInput);
    from_discriminant_macro::codegen::generate(&input).into()
}

#[proc_macro_attribute]
pub fn ffi(attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(input as syn::DeriveInput);
    let input = ffi_macro::parse::FfiInput::from_attr_and_item(attr.into(), item);
    ffi_macro::codegen::generate(&input).into()
}
