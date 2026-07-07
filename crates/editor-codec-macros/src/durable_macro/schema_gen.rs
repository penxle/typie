use proc_macro2::TokenStream;
use quote::quote;

use super::parse::{Field, Grade, Input, Kind};

fn type_string(ty: &syn::Type) -> String {
    quote!(#ty).to_string().replace(' ', "")
}

fn field_schemas(fields: &[Field]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|f| {
            let name = f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();
            let ty = type_string(&f.ty);
            let default = match &f.default {
                None => quote! { ::editor_codec::schema::DefaultSchema::Required },
                Some(None) => quote! { ::editor_codec::schema::DefaultSchema::Trait },
                Some(Some((_expr, raw))) => {
                    // 파스된 Expr는 codegen용, 스키마에는 사용자가 쓴 원문 문자열을 그대로 캡처
                    quote! { ::editor_codec::schema::DefaultSchema::Expr(#raw.to_owned()) }
                }
            };
            quote! {
                ::editor_codec::schema::FieldSchema {
                    name: #name.to_owned(),
                    ty: #ty.to_owned(),
                    default: #default,
                }
            }
        })
        .collect()
}

pub fn generate(input: &Input) -> TokenStream {
    let ident = &input.ident;
    let name = ident.to_string();
    let kind = match &input.kind {
        Kind::Struct(s) => {
            let fields = field_schemas(&s.fields);
            match input.grade {
                Grade::Evolvable => quote! {
                    ::editor_codec::schema::SchemaKind::EvolvableStruct {
                        fields: vec![#(#fields),*],
                    }
                },
                Grade::Frozen => quote! {
                    ::editor_codec::schema::SchemaKind::FrozenStruct {
                        fields: vec![#(#fields),*],
                    }
                },
                _ => unreachable!("struct grades are evolvable|frozen"),
            }
        }
        Kind::Enum(e) => {
            let variants = e.variants.iter().map(|v| {
                let variant_name = v.ident.to_string();
                let tag = v.tag;
                let frozen_payload = v.frozen_payload;
                let fields = field_schemas(&v.fields);
                quote! {
                    ::editor_codec::schema::VariantSchema {
                        name: #variant_name.to_owned(),
                        tag: #tag,
                        frozen_payload: #frozen_payload,
                        fields: vec![#(#fields),*],
                    }
                }
            });
            let retired = &input.retired;
            match input.grade {
                Grade::Open => quote! {
                    ::editor_codec::schema::SchemaKind::OpenEnum {
                        variants: vec![#(#variants),*],
                        retired: vec![#(#retired),*],
                    }
                },
                Grade::Closed => quote! {
                    ::editor_codec::schema::SchemaKind::ClosedEnum {
                        variants: vec![#(#variants),*],
                    }
                },
                _ => unreachable!("enum grades are open|closed"),
            }
        }
    };
    quote! {
        impl ::editor_codec::schema::DurableSchema for #ident {
            fn schema() -> ::editor_codec::schema::TypeSchema {
                ::editor_codec::schema::TypeSchema { name: #name.to_owned(), kind: #kind }
            }
        }
    }
}
