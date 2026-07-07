use proc_macro2::TokenStream;
use quote::quote;

use super::parse::{EnumKind, Field, Grade, Input, Kind, StructKind, Variant};

pub fn generate(input: &Input) -> TokenStream {
    match &input.kind {
        Kind::Struct(s) => generate_struct(input, s),
        Kind::Enum(e) => generate_enum(input, e),
    }
}

fn missing_arm(ty_name: &str, field: &Field) -> TokenStream {
    let field_name = field
        .ident
        .as_ref()
        .map(|i| i.to_string())
        .unwrap_or_default();
    match &field.default {
        None => quote! {
            return ::std::result::Result::Err(
                ::editor_codec::Corruption::MissingRequiredField {
                    ty: #ty_name,
                    field: #field_name,
                }
                .into(),
            )
        },
        Some(None) => quote! { ::std::default::Default::default() },
        Some(Some((expr, _raw))) => quote! { #expr },
    }
}

fn generate_struct(input: &Input, s: &StructKind) -> TokenStream {
    let ident = &input.ident;
    let ty_name = ident.to_string();
    let field_idents: Vec<syn::Ident> = s
        .fields
        .iter()
        .map(|f| f.ident.clone().expect("named field"))
        .collect();
    let field_tys: Vec<syn::Type> = s.fields.iter().map(|f| f.ty.clone()).collect();

    match input.grade {
        Grade::Frozen => quote! {
            impl ::editor_codec::durable::Durable for #ident {
                fn collect(&self, cc: &mut ::editor_codec::ctx::CollectCtx) {
                    #( ::editor_codec::durable::Durable::collect(&self.#field_idents, cc); )*
                }
                fn encode(
                    &self,
                    ctx: &::editor_codec::ctx::EncCtx,
                    out: &mut ::std::vec::Vec<u8>,
                ) -> ::editor_codec::CodecResult<()> {
                    #( ::editor_codec::durable::Durable::encode(&self.#field_idents, ctx, out)?; )*
                    ::std::result::Result::Ok(())
                }
                fn decode(
                    ctx: &::editor_codec::ctx::DecCtx,
                    input: &mut &[u8],
                ) -> ::editor_codec::CodecResult<Self> {
                    #(
                        let #field_idents: #field_tys =
                            ::editor_codec::durable::Durable::decode(ctx, input)?;
                    )*
                    ::std::result::Result::Ok(Self { #(#field_idents),* })
                }
            }
        },
        Grade::Evolvable => {
            let tail = s.tail.as_ref().expect("evolvable struct has tail");
            let decode_fields = s.fields.iter().map(|f| {
                let ident = f.ident.as_ref().expect("named field");
                let ty = &f.ty;
                let missing = missing_arm(&ty_name, f);
                quote! {
                    let #ident: #ty = match frame.try_field(
                        |i| <#ty as ::editor_codec::durable::Durable>::decode(ctx, i),
                    )? {
                        ::std::option::Option::Some(v) => v,
                        ::std::option::Option::None => #missing,
                    };
                }
            });
            quote! {
                impl ::editor_codec::durable::Durable for #ident {
                    fn collect(&self, cc: &mut ::editor_codec::ctx::CollectCtx) {
                        #( ::editor_codec::durable::Durable::collect(&self.#field_idents, cc); )*
                    }
                    fn encode(
                        &self,
                        ctx: &::editor_codec::ctx::EncCtx,
                        out: &mut ::std::vec::Vec<u8>,
                    ) -> ::editor_codec::CodecResult<()> {
                        ::editor_codec::framing::write_frame(out, |body| {
                            #( ::editor_codec::durable::Durable::encode(&self.#field_idents, ctx, body)?; )*
                            ::editor_codec::framing::write_tail(&self.#tail, body);
                            ::std::result::Result::Ok(())
                        })
                    }
                    fn decode(
                        ctx: &::editor_codec::ctx::DecCtx,
                        input: &mut &[u8],
                    ) -> ::editor_codec::CodecResult<Self> {
                        let mut frame = ::editor_codec::framing::FrameReader::open(input)?;
                        #(#decode_fields)*
                        let #tail = ::editor_codec::framing::UnknownTail(frame.capture_tail());
                        ::std::result::Result::Ok(Self { #(#field_idents,)* #tail })
                    }
                }
            }
        }
        _ => unreachable!("struct grades are evolvable|frozen"),
    }
}

fn bindings(variant: &Variant) -> Vec<syn::Ident> {
    variant
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| match &f.ident {
            Some(ident) => ident.clone(),
            None => syn::Ident::new(&format!("f{i}"), proc_macro2::Span::call_site()),
        })
        .collect()
}

fn pattern(variant: &Variant, binds: &[syn::Ident], tail: Option<&syn::Ident>) -> TokenStream {
    if variant.fields.is_empty() && tail.is_none() {
        return quote! {};
    }
    if variant.named {
        let tail_part = tail.map(|t| quote! { #t, });
        quote! { { #(#binds,)* #tail_part } }
    } else {
        quote! { ( #(#binds),* ) }
    }
}

fn generate_enum(input: &Input, e: &EnumKind) -> TokenStream {
    let ident = &input.ident;
    let ty_name = ident.to_string();

    if matches!(input.grade, Grade::Closed) {
        let encode_arms = e.variants.iter().map(|v| {
            let name = &v.ident;
            let tag = v.tag;
            quote! { Self::#name => ::editor_codec::framing::write_closed_tag(#tag, out), }
        });
        let decode_arms = e.variants.iter().map(|v| {
            let name = &v.ident;
            let tag = v.tag;
            quote! { #tag => ::std::result::Result::Ok(Self::#name), }
        });
        return quote! {
            impl ::editor_codec::durable::Durable for #ident {
                fn encode(
                    &self,
                    _ctx: &::editor_codec::ctx::EncCtx,
                    out: &mut ::std::vec::Vec<u8>,
                ) -> ::editor_codec::CodecResult<()> {
                    match self {
                        #(#encode_arms)*
                    }
                    ::std::result::Result::Ok(())
                }
                fn decode(
                    _ctx: &::editor_codec::ctx::DecCtx,
                    input: &mut &[u8],
                ) -> ::editor_codec::CodecResult<Self> {
                    let tag = ::editor_codec::framing::read_closed_tag(input)?;
                    match tag {
                        #(#decode_arms)*
                        n => ::std::result::Result::Err(
                            ::editor_codec::Corruption::UnknownClosedTag { ty: #ty_name, tag: n }
                                .into(),
                        ),
                    }
                }
            }
        };
    }

    let unknown = e.unknown.as_ref().expect("open enum has unknown variant");

    let any_fields = e.variants.iter().any(|v| !v.fields.is_empty());
    let ctx_param = if any_fields {
        quote! { ctx }
    } else {
        quote! { _ctx }
    };
    let cc_param = if any_fields {
        quote! { cc }
    } else {
        quote! { _cc }
    };

    let collect_arms = e.variants.iter().map(|v| {
        let name = &v.ident;
        let binds = bindings(v);
        let pat = pattern(v, &binds, v.tail.as_ref());
        if v.fields.is_empty() && v.tail.is_none() {
            quote! { Self::#name => {}, }
        } else {
            let tail_ignore = v.tail.as_ref().map(|t| quote! { let _ = #t; });
            quote! {
                Self::#name #pat => {
                    #( ::editor_codec::durable::Durable::collect(#binds, cc); )*
                    #tail_ignore
                },
            }
        }
    });

    let encode_arms = e.variants.iter().map(|v| {
        let name = &v.ident;
        let tag = v.tag;
        let binds = bindings(v);
        let pat = pattern(v, &binds, v.tail.as_ref());
        if v.frozen_payload {
            quote! {
                Self::#name #pat => ::editor_codec::framing::write_open_variant(#tag, out, |body| {
                    #( ::editor_codec::durable::Durable::encode(#binds, ctx, body)?; )*
                    ::std::result::Result::Ok(())
                }),
            }
        } else {
            let tail = v.tail.as_ref().expect("evolvable payload has tail");
            quote! {
                Self::#name #pat => ::editor_codec::framing::write_open_variant(#tag, out, |body| {
                    #( ::editor_codec::durable::Durable::encode(#binds, ctx, body)?; )*
                    ::editor_codec::framing::write_tail(#tail, body);
                    ::std::result::Result::Ok(())
                }),
            }
        }
    });

    let decode_arms = e.variants.iter().map(|v| {
        let name = &v.ident;
        let tag = v.tag;
        let binds = bindings(v);
        let tys: Vec<&syn::Type> = v.fields.iter().map(|f| &f.ty).collect();
        if v.frozen_payload {
            let construct = pattern(v, &binds, None);
            quote! {
                #tag => {
                    let mut body = body;
                    #(
                        let #binds: #tys =
                            ::editor_codec::durable::Durable::decode(ctx, &mut body)?;
                    )*
                    ::editor_codec::framing::expect_consumed(body)?;
                    ::std::result::Result::Ok(Self::#name #construct)
                }
            }
        } else {
            let tail = v.tail.as_ref().expect("evolvable payload has tail");
            let ty_lit = ty_name.clone();
            let decode_fields = v.fields.iter().zip(&binds).map(|(f, bind)| {
                let ty = &f.ty;
                let missing = missing_arm(&ty_lit, f);
                quote! {
                    let #bind: #ty = match frame.try_field(
                        |i| <#ty as ::editor_codec::durable::Durable>::decode(ctx, i),
                    )? {
                        ::std::option::Option::Some(v) => v,
                        ::std::option::Option::None => #missing,
                    };
                }
            });
            quote! {
                #tag => {
                    let mut frame = ::editor_codec::framing::FrameReader::from_body(body);
                    #(#decode_fields)*
                    let #tail = ::editor_codec::framing::UnknownTail(frame.capture_tail());
                    ::std::result::Result::Ok(Self::#name { #(#binds,)* #tail })
                }
            }
        }
    });

    quote! {
        impl ::editor_codec::durable::Durable for #ident {
            fn collect(&self, #cc_param: &mut ::editor_codec::ctx::CollectCtx) {
                match self {
                    #(#collect_arms)*
                    Self::#unknown(_) => {},
                }
            }
            fn encode(
                &self,
                #ctx_param: &::editor_codec::ctx::EncCtx,
                out: &mut ::std::vec::Vec<u8>,
            ) -> ::editor_codec::CodecResult<()> {
                match self {
                    #(#encode_arms)*
                    Self::#unknown(u) => ::editor_codec::framing::write_unknown_variant(u, out),
                }
            }
            fn decode(
                #ctx_param: &::editor_codec::ctx::DecCtx,
                input: &mut &[u8],
            ) -> ::editor_codec::CodecResult<Self> {
                let (tag, body) = ::editor_codec::framing::read_open_variant(input)?;
                match tag {
                    #(#decode_arms)*
                    n => ::std::result::Result::Ok(Self::#unknown(
                        ::editor_codec::framing::UnknownPayload { tag: n, bytes: body.to_vec() },
                    )),
                }
            }
        }
    }
}
