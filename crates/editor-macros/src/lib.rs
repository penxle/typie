mod content_macro;
mod context_macro;
mod doc_macro;
mod ffi_export_macro;
mod ffi_macro;
mod from_discriminant_macro;
mod modifier_state_macro;
mod node_attr_macro;
mod node_companion_macro;
mod preamble_macro;
mod state_macro;
mod wire_macro;

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

#[proc_macro_derive(ModifierState)]
pub fn derive_modifier_state(input: TokenStream) -> TokenStream {
    let derive_input = syn::parse_macro_input!(input as syn::DeriveInput);
    let parsed = match modifier_state_macro::parse::ModifierStateInput::from_derive(&derive_input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };
    modifier_state_macro::codegen::generate(&parsed).into()
}

#[proc_macro_derive(NodeCompanion)]
pub fn node_companion(input: TokenStream) -> TokenStream {
    let derive_input = syn::parse_macro_input!(input as syn::DeriveInput);
    let parsed = match node_companion_macro::parse::NodeCompanionInput::from_derive(&derive_input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };
    node_companion_macro::codegen::generate(&parsed).into()
}

#[proc_macro_derive(NodeAttr, attributes(node_attr, plain))]
pub fn node_attr(input: TokenStream) -> TokenStream {
    let derive_input = syn::parse_macro_input!(input as syn::DeriveInput);
    let parsed = match node_attr_macro::parse::NodeAttrInput::from_derive(&derive_input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };
    node_attr_macro::codegen::generate(&parsed).into()
}

#[proc_macro_attribute]
pub fn ffi(attr: TokenStream, input: TokenStream) -> TokenStream {
    // Try as struct/enum first
    if let Ok(item) = syn::parse::<syn::DeriveInput>(input.clone()) {
        let input = ffi_macro::parse::FfiInput::from_attr_and_item(attr.into(), item);
        return ffi_macro::codegen::generate(&input).into();
    }
    // Try as type alias
    if let Ok(item) = syn::parse::<syn::ItemType>(input) {
        return ffi_macro::codegen::generate_type_alias(&item).into();
    }
    panic!("#[ffi] can only be applied to structs, enums, or type aliases");
}

#[proc_macro_attribute]
pub fn ffi_export(attr: TokenStream, input: TokenStream) -> TokenStream {
    let mode = syn::parse_macro_input!(attr as ffi_export_macro::parse::FfiExportMode);
    let item = syn::parse_macro_input!(input as syn::ItemImpl);
    ffi_export_macro::codegen::generate(mode, item).into()
}

#[proc_macro_derive(Wire, attributes(wire))]
pub fn derive_wire(input: TokenStream) -> TokenStream {
    let derive_input = syn::parse_macro_input!(input as syn::DeriveInput);
    let parsed = match wire_macro::parse::WireInput::from_derive(&derive_input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };
    wire_macro::codegen::generate(&parsed).into()
}
