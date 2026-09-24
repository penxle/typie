use editor_bindgen::meta::FfiInterface;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemImpl, Type, TypePath, visit_mut::VisitMut};

use super::parse::FfiExportMode;

pub fn generate(mode: FfiExportMode, mut item: ItemImpl) -> TokenStream {
    let iface = super::meta::extract(&item);
    let meta_static = generate_iface_static(&iface);

    ComplexRewriter { mode }.visit_item_impl_mut(&mut item);
    if let FfiExportMode::Wasm = mode {
        name_primary_constructors(&mut item, &iface);
    }

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

fn name_primary_constructors(item: &mut ItemImpl, iface: &FfiInterface) {
    let methods = item.items.iter_mut().filter_map(|item| match item {
        syn::ImplItem::Fn(method) => Some(method),
        _ => None,
    });
    for method in methods {
        let Some(meta) = iface
            .methods
            .iter()
            .find(|meta| method.sig.ident == meta.name)
        else {
            continue;
        };
        if meta.is_primary_constructor() {
            let js_name = format_ident!("{}", meta.wrapper_name());
            method
                .attrs
                .push(syn::parse_quote!(#[wasm_bindgen(js_name = #js_name)]));
        }
    }
}

fn generate_iface_static(iface: &FfiInterface) -> TokenStream {
    let first_method = iface
        .methods
        .first()
        .map(|method| method.name.clone())
        .unwrap_or_else(|| "opaque".into());

    let encoded = bitcode::encode(iface);
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

        if let Type::Path(TypePath {
            path, qself: None, ..
        }) = ty
        {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn host_impl() -> ItemImpl {
        syn::parse_quote! {
            impl EditorHost {
                #[cfg_attr(feature = "uniffi", uniffi::constructor)]
                pub fn new(icu_data: Vec<u8>) -> EditorResult<Owned<Self>> {
                    unimplemented!()
                }

                pub fn set_fonts(&self, families: Vec<Complex<FontFamily>>) {}
            }
        }
    }

    #[test]
    fn wasm_exposes_the_primary_constructor_as_create() {
        let output = generate(FfiExportMode::Wasm, host_impl()).to_string();
        let attribute = output.find("js_name = create").expect(&output);
        let constructor = output.find("pub fn new").expect(&output);
        assert!(attribute < constructor, "{output}");
        assert_eq!(output.matches("js_name").count(), 1, "{output}");
    }

    #[test]
    fn uniffi_leaves_the_primary_constructor_to_uniffi() {
        let output = generate(FfiExportMode::Uniffi, host_impl()).to_string();
        assert!(!output.contains("js_name"), "{output}");
    }

    #[test]
    fn primary_constructor_is_matched_by_name_not_position() {
        let mut item: ItemImpl = syn::parse_quote! {
            impl EditorHost {
                pub fn set_fonts(&self, families: Vec<Complex<FontFamily>>) {}

                #[cfg_attr(feature = "uniffi", uniffi::constructor)]
                pub fn new(icu_data: Vec<u8>) -> EditorResult<Owned<Self>> {
                    unimplemented!()
                }
            }
        };
        let mut iface = super::super::meta::extract(&item);
        iface.methods.retain(|method| method.is_constructor);
        name_primary_constructors(&mut item, &iface);
        let output = quote!(#item).to_string();
        let setter = output.find("pub fn set_fonts").expect(&output);
        let attribute = output.find("js_name = create").expect(&output);
        let constructor = output.find("pub fn new").expect(&output);
        assert!(setter < attribute && attribute < constructor, "{output}");
        assert_eq!(output.matches("js_name").count(), 1, "{output}");
    }
}
