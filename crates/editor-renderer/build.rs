use heck::{ToShoutySnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=../../assets/theme.json");

    let json_str =
        fs::read_to_string("../../assets/theme.json").expect("failed to read theme.json");
    let json: serde_json::Value =
        serde_json::from_str(&json_str).expect("failed to parse theme.json");

    let shared = json["shared"].as_object().expect("missing shared");
    let light_shared = json["lightShared"]
        .as_object()
        .expect("missing lightShared");
    let dark_shared = json["darkShared"].as_object().expect("missing darkShared");
    let variants = json["variants"].as_object().expect("missing variants");

    let variant_idents: Vec<_> = variants
        .keys()
        .map(|k| format_ident!("{}", k.to_upper_camel_case()))
        .collect();

    let static_idents: Vec<_> = variants
        .keys()
        .map(|k| format_ident!("{}", k.to_shouty_snake_case()))
        .collect();

    let map_statics: Vec<TokenStream> = variants
        .iter()
        .zip(&static_idents)
        .map(|((key, variant_colors), static_ident)| {
            let variant_obj = variant_colors.as_object().expect("variant must be object");
            let is_light = key.starts_with("light-");
            let mode_shared = if is_light { light_shared } else { dark_shared };

            let entries: Vec<TokenStream> = shared
                .iter()
                .chain(mode_shared.iter())
                .chain(variant_obj.iter())
                .map(|(token, hex)| {
                    let (r, g, b, a) = parse_hex_color(hex.as_str().unwrap());
                    quote! { #token => Color::new(#r, #g, #b, #a) }
                })
                .collect();

            quote! {
                static #static_ident: phf::Map<&'static str, Color> = phf::phf_map! {
                    #(#entries,)*
                };
            }
        })
        .collect();

    let output = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum ThemeVariant {
            #(#variant_idents,)*
        }

        #(#map_statics)*

        impl ThemeVariant {
            pub fn colors(&self) -> &'static phf::Map<&'static str, Color> {
                match self {
                    #(Self::#variant_idents => &#static_idents,)*
                }
            }
        }
    };

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("theme_data.rs");
    fs::write(&dest_path, output.to_string()).unwrap();
}

fn parse_hex_color(hex: &str) -> (u8, u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    match hex.len() {
        6 => (
            u8::from_str_radix(&hex[0..2], 16).unwrap(),
            u8::from_str_radix(&hex[2..4], 16).unwrap(),
            u8::from_str_radix(&hex[4..6], 16).unwrap(),
            255,
        ),
        8 => (
            u8::from_str_radix(&hex[0..2], 16).unwrap(),
            u8::from_str_radix(&hex[2..4], 16).unwrap(),
            u8::from_str_radix(&hex[4..6], 16).unwrap(),
            u8::from_str_radix(&hex[6..8], 16).unwrap(),
        ),
        _ => panic!("invalid hex color: #{hex}"),
    }
}
