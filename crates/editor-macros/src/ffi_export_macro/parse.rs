use syn::{Ident, parse::Parse};

#[derive(Clone, Copy)]
pub enum FfiExportMode {
    Uniffi,
    Wasm,
}

impl Parse for FfiExportMode {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "uniffi" => Ok(Self::Uniffi),
            "wasm" => Ok(Self::Wasm),
            other => Err(syn::Error::new(
                ident.span(),
                format!("expected `uniffi` or `wasm`, found `{other}`"),
            )),
        }
    }
}
