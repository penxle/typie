use proc_macro2::TokenStream;
use quote::quote;

use super::parse::{WireField, WireInput, WireKind, WireVariant};

pub fn generate(input: &WireInput) -> TokenStream {
    let ident = &input.ident;

    let mut generics = input.generics.clone();
    for tp in generics.type_params_mut() {
        tp.bounds.push(syn::parse_quote!(::editor_crdt::wire::Wire));
    }
    let (impl_g, ty_g, where_g) = generics.split_for_impl();

    match &input.kind {
        WireKind::Enum(e) => {
            let ty_name_lit = ident.to_string();

            // Empty enum: `match self { ... }` on `&Self` is non-exhaustive because
            // references are always considered inhabited. `match *self {}` on the
            // uninhabited value type is well-formed and proves no input can reach the body.
            if e.variants.is_empty() {
                return quote! {
                    impl #impl_g ::editor_crdt::wire::Wire for #ident #ty_g #where_g {
                        fn collect(&self, _ctx: &mut ::editor_crdt::wire::CollectCtx) {
                            match *self {}
                        }
                        fn encode(
                            &self,
                            _ctx: &::editor_crdt::wire::EncCtx,
                            _out: &mut ::std::vec::Vec<u8>,
                        ) -> ::editor_crdt::wire::WireResult<()> {
                            match *self {}
                        }
                        fn decode(
                            _ctx: &::editor_crdt::wire::DecCtx,
                            _input: &mut &[u8],
                        ) -> ::editor_crdt::wire::WireResult<Self> {
                            ::std::result::Result::Err(
                                ::editor_crdt::wire::WireError::UnknownVariant {
                                    ty: #ty_name_lit,
                                    tag: 0,
                                },
                            )
                        }
                    }
                };
            }

            let collect_arms = e.variants.iter().map(collect_arm_for_variant);
            let encode_arms = e.variants.iter().map(encode_arm_for_variant);
            let decode_arms = e.variants.iter().map(decode_arm_for_variant);

            quote! {
                impl #impl_g ::editor_crdt::wire::Wire for #ident #ty_g #where_g {
                    fn collect(&self, ctx: &mut ::editor_crdt::wire::CollectCtx) {
                        match self {
                            #(#collect_arms)*
                        }
                    }
                    fn encode(
                        &self,
                        ctx: &::editor_crdt::wire::EncCtx,
                        out: &mut ::std::vec::Vec<u8>,
                    ) -> ::editor_crdt::wire::WireResult<()> {
                        match self {
                            #(#encode_arms)*
                        }
                        Ok(())
                    }
                    fn decode(
                        ctx: &::editor_crdt::wire::DecCtx,
                        input: &mut &[u8],
                    ) -> ::editor_crdt::wire::WireResult<Self> {
                        let tag = <u8 as ::editor_crdt::wire::Wire>::decode(ctx, input)?;
                        match tag {
                            #(#decode_arms)*
                            n => ::std::result::Result::Err(
                                ::editor_crdt::wire::WireError::UnknownVariant {
                                    ty: #ty_name_lit,
                                    tag: n,
                                },
                            ),
                        }
                    }
                }
            }
        }
        WireKind::Struct(s) => {
            if s.transparent {
                let inner_ty = &s.fields[0].ty;
                quote! {
                    impl #impl_g ::editor_crdt::wire::Wire for #ident #ty_g #where_g {
                        fn collect(&self, ctx: &mut ::editor_crdt::wire::CollectCtx) {
                            ::editor_crdt::wire::Wire::collect(&self.0, ctx);
                        }
                        fn encode(
                            &self,
                            ctx: &::editor_crdt::wire::EncCtx,
                            out: &mut ::std::vec::Vec<u8>,
                        ) -> ::editor_crdt::wire::WireResult<()> {
                            <#inner_ty as ::editor_crdt::wire::Wire>::encode(&self.0, ctx, out)
                        }
                        fn decode(
                            ctx: &::editor_crdt::wire::DecCtx,
                            input: &mut &[u8],
                        ) -> ::editor_crdt::wire::WireResult<Self> {
                            let inner = <#inner_ty as ::editor_crdt::wire::Wire>::decode(ctx, input)?;
                            ::std::result::Result::Ok(Self(inner))
                        }
                    }
                }
            } else {
                let collect_calls = s.fields.iter().filter(|f| !f.skip).map(|f| {
                    let id = f.ident.as_ref().unwrap();
                    quote! { ::editor_crdt::wire::Wire::collect(&self.#id, ctx); }
                });
                let encode_calls = s.fields.iter().filter(|f| !f.skip).map(|f| {
                    let id = f.ident.as_ref().unwrap();
                    quote! { ::editor_crdt::wire::Wire::encode(&self.#id, ctx, out)?; }
                });
                let decode_lets = s.fields.iter().map(|f| {
                    let id = f.ident.as_ref().unwrap();
                    if f.skip {
                        quote! { let #id = ::std::default::Default::default(); }
                    } else {
                        let ty = &f.ty;
                        quote! { let #id = <#ty as ::editor_crdt::wire::Wire>::decode(ctx, input)?; }
                    }
                });
                let field_names: Vec<_> =
                    s.fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
                quote! {
                    impl #impl_g ::editor_crdt::wire::Wire for #ident #ty_g #where_g {
                        fn collect(&self, ctx: &mut ::editor_crdt::wire::CollectCtx) {
                            #(#collect_calls)*
                        }
                        fn encode(
                            &self,
                            ctx: &::editor_crdt::wire::EncCtx,
                            out: &mut ::std::vec::Vec<u8>,
                        ) -> ::editor_crdt::wire::WireResult<()> {
                            #(#encode_calls)*
                            Ok(())
                        }
                        fn decode(
                            ctx: &::editor_crdt::wire::DecCtx,
                            input: &mut &[u8],
                        ) -> ::editor_crdt::wire::WireResult<Self> {
                            #(#decode_lets)*
                            ::std::result::Result::Ok(Self { #(#field_names),* })
                        }
                    }
                }
            }
        }
    }
}

fn field_binding(f: &WireField, idx: usize) -> syn::Ident {
    match &f.ident {
        Some(id) => id.clone(),
        None => syn::Ident::new(&format!("f{idx}"), proc_macro2::Span::call_site()),
    }
}

fn field_destructure_pattern(fields: &[WireField]) -> TokenStream {
    if fields.is_empty() {
        return quote! {};
    }
    let binds: Vec<syn::Ident> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| field_binding(f, i))
        .collect();
    if fields.iter().all(|f| f.ident.is_some()) {
        quote! { { #(#binds),* } }
    } else {
        quote! { ( #(#binds),* ) }
    }
}

fn collect_arm_for_variant(v: &WireVariant) -> TokenStream {
    let name = &v.ident;
    if v.fields.is_empty() {
        return quote! { Self::#name => {}, };
    }
    let pat = field_destructure_pattern(&v.fields);
    let calls = v
        .fields
        .iter()
        .enumerate()
        .filter(|(_, f)| !f.skip)
        .map(|(i, f)| {
            let bind = field_binding(f, i);
            quote! { ::editor_crdt::wire::Wire::collect(#bind, ctx); }
        });
    quote! { Self::#name #pat => { #(#calls)* }, }
}

fn encode_arm_for_variant(v: &WireVariant) -> TokenStream {
    let name = &v.ident;
    let tag = v.tag;
    if v.fields.is_empty() {
        return quote! { Self::#name => { out.push(#tag); } };
    }
    let pat = field_destructure_pattern(&v.fields);
    let calls = v
        .fields
        .iter()
        .enumerate()
        .filter(|(_, f)| !f.skip)
        .map(|(i, f)| {
            let bind = field_binding(f, i);
            quote! { ::editor_crdt::wire::Wire::encode(#bind, ctx, out)?; }
        });
    quote! {
        Self::#name #pat => {
            out.push(#tag);
            #(#calls)*
        }
    }
}

fn decode_arm_for_variant(v: &WireVariant) -> TokenStream {
    let name = &v.ident;
    let tag = v.tag;
    if v.fields.is_empty() {
        return quote! { #tag => ::std::result::Result::Ok(Self::#name), };
    }
    let decoded = v
        .fields
        .iter()
        .enumerate()
        .filter(|(_, f)| !f.skip)
        .map(|(i, f)| {
            let bind = field_binding(f, i);
            let ty = &f.ty;
            quote! {
                let #bind = <#ty as ::editor_crdt::wire::Wire>::decode(ctx, input)?;
            }
        });
    let constructor = construct_variant(v);
    quote! {
        #tag => {
            #(#decoded)*
            ::std::result::Result::Ok(Self::#name #constructor)
        }
    }
}

fn construct_variant(v: &WireVariant) -> TokenStream {
    if v.fields.is_empty() {
        return quote! {};
    }
    let binds: Vec<syn::Ident> = v
        .fields
        .iter()
        .enumerate()
        .filter(|(_, f)| !f.skip)
        .map(|(i, f)| field_binding(f, i))
        .collect();
    if v.fields.iter().all(|f| f.ident.is_some()) {
        quote! { { #(#binds),* } }
    } else {
        quote! { ( #(#binds),* ) }
    }
}
