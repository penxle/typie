use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::{EnumVariant, FfiInput, FfiTypeKind, StructField};

pub fn generate(input: &FfiInput) -> TokenStream {
    let name = &input.item.ident;
    let describe_name = format_ident!("__ffi_describe_{}", name);
    let kind = input.kind();

    let descriptor = match kind {
        FfiTypeKind::Custom { target } => {
            quote! { @custom_type #name = $crate :: #name : #target ; }
        }
        FfiTypeKind::Struct { fields } => generate_struct_descriptor(name, &fields),
        FfiTypeKind::Enum { variants } => generate_enum_descriptor(name, &variants),
    };

    let original_item = &input.item;

    quote! {
        #original_item

        #[macro_export]
        macro_rules! #describe_name {
            ($callback:path) => {
                $callback! {
                    #descriptor
                }
            };
        }
    }
}

fn generate_struct_descriptor(name: &syn::Ident, fields: &[StructField]) -> TokenStream {
    let field_entries: Vec<_> = fields
        .iter()
        .map(|f| {
            let fname = &f.name;
            let fty = &f.ty;
            quote! { @field #fname : #fty ; }
        })
        .collect();

    quote! {
        @struct #name = $crate :: #name ;
        #(#field_entries)*
        @end;
    }
}

fn generate_enum_descriptor(name: &syn::Ident, variants: &[EnumVariant]) -> TokenStream {
    let variant_entries: Vec<_> = variants
        .iter()
        .map(|v| match v {
            EnumVariant::Unit { name: vname } => {
                quote! { @unit #vname = $crate :: #name :: #vname ; }
            }
            EnumVariant::Tuple {
                name: vname,
                fields,
            } => {
                let bindings: Vec<_> = fields
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| {
                        let var = format_ident!("_{}", i);
                        quote! { #var : #ty }
                    })
                    .collect();
                quote! { @tuple #vname ( #(#bindings),* ) = $crate :: #name :: #vname ; }
            }
            EnumVariant::Struct {
                name: vname,
                fields,
            } => {
                let field_defs: Vec<_> = fields
                    .iter()
                    .map(|f| {
                        let fname = &f.name;
                        let fty = &f.ty;
                        quote! { #fname : #fty }
                    })
                    .collect();
                quote! { @named #vname { #(#field_defs),* } = $crate :: #name :: #vname ; }
            }
        })
        .collect();

    quote! {
        @enum #name = $crate :: #name ;
        #(#variant_entries)*
        @end;
    }
}
