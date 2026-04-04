use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemImpl, Type, TypePath, visit_mut::VisitMut};

use super::parse::FfiExportMode;

pub fn generate(mode: FfiExportMode, mut item: ItemImpl) -> TokenStream {
    // Extract metadata BEFORE ComplexRewriter modifies the item
    let meta_static = generate_iface_static(&item);

    ComplexRewriter { mode }.visit_item_impl_mut(&mut item);

    let attr = match mode {
        FfiExportMode::Uniffi => quote! { #[::uniffi::export] },
        FfiExportMode::Wasm => quote! { #[::wasm_bindgen::prelude::wasm_bindgen] },
    };

    quote! {
        #attr
        #item

        #meta_static
    }
}

fn generate_iface_static(item: &ItemImpl) -> TokenStream {
    let iface = super::meta::extract(item);

    let first_method = match iface.methods.first() {
        Some(m) => m.name.clone(),
        None => return TokenStream::new(),
    };

    let encoded = bitcode::encode(&iface);
    let payload_len = encoded.len() as u32;
    let total_len = 4 + encoded.len();
    let prefix = payload_len.to_le_bytes();
    let all_bytes: Vec<u8> = prefix.iter().copied().chain(encoded).collect();

    let crate_name = std::env::var("CARGO_PKG_NAME")
        .unwrap_or_default()
        .replace('-', "_");
    let ident = format_ident!("FFI_IFACE_{}_{}_{}", crate_name, iface.name, first_method);

    quote! {
        #[used]
        #[unsafe(no_mangle)]
        pub static #ident: [u8; #total_len] = [#(#all_bytes),*];
    }
}

struct ComplexRewriter {
    mode: FfiExportMode,
}

impl VisitMut for ComplexRewriter {
    fn visit_type_mut(&mut self, ty: &mut Type) {
        syn::visit_mut::visit_type_mut(self, ty);

        if let Type::Path(TypePath { path, qself: None }) = ty {
            let last = match path.segments.last() {
                Some(seg) if seg.ident == "Complex" => seg,
                _ => return,
            };

            let inner = match &last.arguments {
                syn::PathArguments::AngleBracketed(args) if args.args.len() == 1 => {
                    match args.args.first() {
                        Some(syn::GenericArgument::Type(inner)) => inner.clone(),
                        _ => return,
                    }
                }
                _ => return,
            };

            *ty = match self.mode {
                FfiExportMode::Uniffi => syn::parse_quote! { String },
                FfiExportMode::Wasm => syn::parse_quote! { ::tsify::Ts<#inner> },
            };
        }
    }
}
