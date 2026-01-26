use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Fields, Ident, ItemFn, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

mod icon;

#[proc_macro]
pub fn svg_icon_path(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as icon::SvgIconArgs);

    match icon::generate_svg_icon_path(&args) {
        Ok(tokens) => tokens.into(),
        Err(e) => syn::Error::new(args.path.span(), e)
            .to_compile_error()
            .into(),
    }
}

#[proc_macro_attribute]
pub fn command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let exec_name = Ident::new(&format!("exec_{}", fn_name), fn_name.span());

    let all_params = &input_fn.sig.inputs;
    let body = &input_fn.block;
    let return_type = &input_fn.sig.output;
    let vis = &input_fn.vis;

    let expanded = quote! {
        #vis fn #exec_name(#all_params) #return_type {
            #body
        }
    };

    TokenStream::from(expanded)
}

struct CommandSpec {
    module: Ident,
    commands: Vec<CommandDef>,
}

struct CommandDef {
    name: Ident,
    params: Vec<(Ident, Type)>,
}

impl Parse for CommandSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![mod]>()?;
        let module: Ident = input.parse()?;

        let content;
        syn::braced!(content in input);

        let mut commands = Vec::new();
        while !content.is_empty() {
            let name: Ident = content.parse()?;

            let params_content;
            syn::parenthesized!(params_content in content);

            let mut params = Vec::new();
            while !params_content.is_empty() {
                let param_name: Ident = params_content.parse()?;
                params_content.parse::<Token![:]>()?;
                let param_type: Type = params_content.parse()?;
                params.push((param_name, param_type));

                if !params_content.is_empty() {
                    params_content.parse::<Token![,]>()?;
                }
            }

            commands.push(CommandDef { name, params });

            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(CommandSpec { module, commands })
    }
}

struct AggregateInput {
    specs: Vec<CommandSpec>,
}

impl Parse for AggregateInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut specs = Vec::new();
        while !input.is_empty() {
            specs.push(input.parse()?);
            if !input.is_empty() {
                let _ = input.parse::<Token![,]>();
            }
        }
        Ok(AggregateInput { specs })
    }
}

#[proc_macro]
pub fn aggregate_commands(input: TokenStream) -> TokenStream {
    let AggregateInput { specs } = parse_macro_input!(input as AggregateInput);

    let mut enum_variants = Vec::new();
    let mut match_arms = Vec::new();
    let mut factory_methods = Vec::new();

    for spec in specs {
        let module = &spec.module;

        for cmd in spec.commands {
            let name = &cmd.name;
            let exec_name = format_ident!("exec_{}", name);
            let variant_name = to_pascal_case(&name.to_string());
            let variant_ident = Ident::new(&variant_name, name.span());

            if cmd.params.is_empty() {
                enum_variants.push(quote! { #variant_ident });
                match_arms.push(quote! {
                    Command::#variant_ident => {
                        #module::#exec_name(tr)
                    }
                });
                factory_methods.push(quote! {
                    pub fn #name() -> Self {
                        Command::#variant_ident
                    }
                });
            } else {
                let param_names: Vec<_> = cmd.params.iter().map(|(n, _)| n).collect();
                let param_types: Vec<_> = cmd.params.iter().map(|(_, t)| t).collect();

                enum_variants.push(quote! {
                    #variant_ident { #(#param_names: #param_types),* }
                });

                match_arms.push(quote! {
                    Command::#variant_ident { #(#param_names),* } => {
                        #module::#exec_name(tr, #(#param_names),*)
                    }
                });

                factory_methods.push(quote! {
                    pub fn #name(#(#param_names: #param_types),*) -> Self {
                        Command::#variant_ident {
                            #(#param_names),*
                        }
                    }
                });
            }
        }
    }

    let expanded = quote! {
        #[derive(Debug, Clone)]
        pub enum Command {
            #(#enum_variants,)*
            Chain(Vec<Command>),
            First(Vec<Command>),
        }

        impl Command {
            #(#factory_methods)*

            pub fn execute(self, tr: &mut Transaction) -> CommandResult {
                match self {
                    #(#match_arms),*
                    Command::Chain(cmds) => {
                        for cmd in cmds {
                            match cmd.execute(tr) {
                                Ok(_) => {},
                                Err(e) => match e.downcast::<CommandError>() {
                                    Ok(e) => anyhow::bail!(e),
                                    Err(e) => return Err(e),
                                },
                            }
                        }

                        Ok(())
                    }
                    Command::First(cmds) => {
                        let checkpoint = tr.checkpoint();
                        for cmd in cmds {
                            match cmd.execute(tr) {
                                Ok(_) => return Ok(()),
                                Err(e) => {
                                    tr.restore(&checkpoint);
                                    match e.downcast::<CommandError>() {
                                        Ok(CommandError::NotApplicable) => continue,
                                        Ok(e) => anyhow::bail!(e),
                                        Err(e) => return Err(e),
                                    }
                                },
                            }
                        }

                        tr.restore(&checkpoint);
                        anyhow::bail!(CommandError::NotApplicable);
                    }
                }
            }
        }
    };

    TokenStream::from(expanded)
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

#[proc_macro_derive(Codec)]
pub fn derive_codec(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match &input.data {
        Data::Struct(data) => derive_struct_codec(&input, data),
        Data::Enum(data) => derive_enum_codec(&input, data),
        _ => panic!("Codec can only be derived for structs and enums"),
    }
}

fn derive_struct_codec(input: &DeriveInput, data: &syn::DataStruct) -> TokenStream {
    let name = &input.ident;

    match &data.fields {
        Fields::Named(fields) => {
            let field_count = fields.named.len();

            if field_count == 1 {
                let field = fields.named.first().unwrap();
                let field_name = &field.ident;
                let field_type = &field.ty;

                let expanded = quote! {
                    impl crate::model::Codec for #name {
                        fn to_value(&self) -> Option<loro::LoroValue> {
                            <#field_type as crate::model::Codec>::to_value(&self.#field_name)
                        }

                        fn from_value(value: loro::LoroValue) -> anyhow::Result<Self> {
                            Ok(Self {
                                #field_name: <#field_type as crate::model::Codec>::from_value(value)?
                            })
                        }

                        fn encode(&mut self, map: &loro::LoroMap) -> anyhow::Result<()> {
                            use crate::model::Codec;
                            let field_name_str = stringify!(#field_name);
                            self.#field_name.encode_field(map, field_name_str)?;
                            Ok(())
                        }

                        fn decode(map: &loro::LoroMap) -> anyhow::Result<Self> {
                            use crate::model::Codec;
                            let field_name_str = stringify!(#field_name);
                            Ok(Self {
                                #field_name: <#field_type>::decode_field(map, field_name_str)?
                            })
                        }
                    }
                };

                TokenStream::from(expanded)
            } else {
                let encode_fields = fields.named.iter().map(|f| {
                    let field_name = &f.ident;
                    let field_name_str = field_name.as_ref().unwrap().to_string();

                    quote! {
                        self.#field_name.encode_field(map, #field_name_str)?;
                    }
                });

                let decode_fields = fields.named.iter().map(|f| {
                    let field_name = &f.ident;
                    let field_name_str = field_name.as_ref().unwrap().to_string();
                    let field_type = &f.ty;

                    quote! {
                        #field_name: <#field_type>::decode_field(map, #field_name_str)?
                    }
                });

                let expanded = quote! {
                    impl crate::model::Codec for #name {
                        fn encode(&mut self, map: &loro::LoroMap) -> anyhow::Result<()> {
                            #(#encode_fields)*
                            Ok(())
                        }

                        fn decode(map: &loro::LoroMap) -> anyhow::Result<Self> {
                            Ok(Self {
                                #(#decode_fields),*
                            })
                        }
                    }
                };

                TokenStream::from(expanded)
            }
        }
        Fields::Unit => {
            let expanded = quote! {
                impl crate::model::Codec for #name {
                    fn to_value(&self) -> Option<loro::LoroValue> {
                        Some(loro::LoroValue::Bool(true))
                    }

                    fn from_value(value: loro::LoroValue) -> anyhow::Result<Self> {
                        match value {
                            loro::LoroValue::Bool(true) => Ok(Self),
                            _ => anyhow::bail!("expected true for unit struct"),
                        }
                    }

                    fn encode(&mut self, _map: &loro::LoroMap) -> anyhow::Result<()> {
                        Ok(())
                    }

                    fn decode(_map: &loro::LoroMap) -> anyhow::Result<Self> {
                        Ok(Self)
                    }
                }
            };

            TokenStream::from(expanded)
        }
        _ => panic!("Codec only supports structs with named fields or unit structs"),
    }
}

fn derive_enum_codec(input: &DeriveInput, data: &syn::DataEnum) -> TokenStream {
    let name = &input.ident;

    let all_unit_variants = data
        .variants
        .iter()
        .all(|v| matches!(v.fields, Fields::Unit));

    if all_unit_variants {
        let to_value_arms = data.variants.iter().map(|variant| {
            let variant_name = &variant.ident;
            let variant_name_snake = variant_name.to_string().to_snake_case();
            quote! {
                #name::#variant_name => Some(loro::LoroValue::String(#variant_name_snake.into()))
            }
        });

        let from_value_arms = data.variants.iter().map(|variant| {
            let variant_name = &variant.ident;
            let variant_name_snake = variant_name.to_string().to_snake_case();
            quote! {
                #variant_name_snake => Ok(#name::#variant_name)
            }
        });

        let default_variant = data
            .variants
            .iter()
            .find(|v| v.attrs.iter().any(|attr| attr.path().is_ident("default")));

        let decode_field_impl = if let Some(variant) = default_variant {
            let variant_name = &variant.ident;
            quote! {
                fn decode_field(map: &loro::LoroMap, key: &str) -> anyhow::Result<Self> {
                    match map.get(key) {
                        Some(value_or_container) => {
                            if let Ok(value) = value_or_container.into_value() {
                                return Self::from_value(value);
                            }
                            Ok(#name::#variant_name)
                        }
                        None => Ok(#name::#variant_name),
                    }
                }
            }
        } else {
            quote! {}
        };

        let expanded = quote! {
            impl crate::model::Codec for #name {
                fn to_value(&self) -> Option<loro::LoroValue> {
                    match self {
                        #(#to_value_arms),*
                    }
                }

                fn from_value(value: loro::LoroValue) -> anyhow::Result<Self> {
                    let s = match value {
                        loro::LoroValue::String(s) => s.to_string(),
                        _ => anyhow::bail!("expected string"),
                    };
                    match s.as_str() {
                        #(#from_value_arms,)*
                        v => anyhow::bail!("unknown variant: {}", v),
                    }
                }

                fn encode(&mut self, _map: &loro::LoroMap) -> anyhow::Result<()> {
                    Ok(())
                }

                fn decode(_map: &loro::LoroMap) -> anyhow::Result<Self> {
                    anyhow::bail!("unit enum should use from_value")
                }

                #decode_field_impl
            }
        };

        return TokenStream::from(expanded);
    }

    let encode_arms = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_name_snake = variant_name.to_string().to_snake_case();

        match &variant.fields {
            Fields::Unit => {
                quote! {
                    #name::#variant_name => {
                        map.insert("type", #variant_name_snake)?;
                    }
                }
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                quote! {
                    #name::#variant_name(inner) => {
                        map.insert("type", #variant_name_snake)?;
                        inner.encode(map)?;
                    }
                }
            }
            Fields::Named(fields) => {
                let field_names: Vec<_> = fields.named.iter().map(|f| &f.ident).collect();
                let encode_fields = fields.named.iter().map(|f| {
                    let field_name = &f.ident;
                    let field_name_str = field_name.as_ref().unwrap().to_string();
                    quote! {
                        #field_name.encode_field(map, #field_name_str)?;
                    }
                });
                quote! {
                    #name::#variant_name { #(#field_names),* } => {
                        map.insert("type", #variant_name_snake)?;
                        #(#encode_fields)*
                    }
                }
            }
            _ => panic!("Codec does not support this variant type"),
        }
    });

    let decode_arms = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_name_snake = variant_name.to_string().to_snake_case();

        match &variant.fields {
            Fields::Unit => {
                quote! {
                    #variant_name_snake => Ok(#name::#variant_name)
                }
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field_type = &fields.unnamed[0].ty;
                quote! {
                    #variant_name_snake => Ok(#name::#variant_name(#field_type::decode(map)?))
                }
            }
            Fields::Named(fields) => {
                let decode_fields = fields.named.iter().map(|f| {
                    let field_name = &f.ident;
                    let field_name_str = field_name.as_ref().unwrap().to_string();
                    let field_type = &f.ty;
                    quote! {
                        #field_name: <#field_type>::decode_field(map, #field_name_str)?
                    }
                });
                quote! {
                    #variant_name_snake => Ok(#name::#variant_name { #(#decode_fields),* })
                }
            }
            _ => panic!("Codec does not support this variant type"),
        }
    });

    let expanded = quote! {
        impl crate::model::Codec for #name {
            fn encode(&mut self, map: &loro::LoroMap) -> anyhow::Result<()> {
                use anyhow::Context;
                match self {
                    #(#encode_arms)*
                }
                Ok(())
            }

            fn decode(map: &loro::LoroMap) -> anyhow::Result<Self> {
                use anyhow::Context;
                let type_value = map.get("type")
                    .context("missing type")?
                    .into_value()
                    .ok()
                    .context("type not a value")?;

                let type_str = match type_value {
                    loro::LoroValue::String(s) => s.to_string(),
                    _ => anyhow::bail!("type not string"),
                };

                match type_str.as_str() {
                    #(#decode_arms,)*
                    v => anyhow::bail!("unknown variant: {}", v),
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(LoroMark)]
pub fn derive_loro_mark(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = &input.ident;
    let mark_type_name = format_ident!("{}Type", enum_name);

    let Data::Enum(data_enum) = &input.data else {
        return syn::Error::new_spanned(&input.ident, "LoroMark can only be derived for enums")
            .to_compile_error()
            .into();
    };

    let mark_type_key_arms = data_enum.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let key = variant_name.to_string().to_snake_case();
        quote! { #mark_type_name::#variant_name => #key }
    });

    let to_value_arms = data_enum.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        quote! {
            #enum_name::#variant_name(inner) => {
                use crate::model::Codec;
                inner.to_value().unwrap_or(loro::LoroValue::Bool(true))
            }
        }
    });

    let from_value_arms = data_enum.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let key = variant_name.to_string().to_snake_case();

        let inner_type = match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => &fields.unnamed[0].ty,
            _ => {
                return syn::Error::new_spanned(
                    variant,
                    "LoroMark variants must have exactly one unnamed field",
                )
                .to_compile_error();
            }
        };

        quote! {
            #key => {
                use crate::model::Codec;
                if matches!(value, loro::LoroValue::Bool(true)) {
                    <#inner_type>::from_value(value).ok()
                        .or_else(|| Some(Default::default()))
                        .map(|inner| #enum_name::#variant_name(inner))
                } else {
                    <#inner_type>::from_value(value).ok()
                        .map(|inner| #enum_name::#variant_name(inner))
                }
            }
        }
    });

    let expanded = quote! {
        impl #mark_type_name {
            pub fn key(&self) -> &'static str {
                match self {
                    #(#mark_type_key_arms),*
                }
            }
        }

        impl #enum_name {
            pub fn key(&self) -> &'static str {
                self.as_type().key()
            }

            pub fn to_loro_value(&self) -> loro::LoroValue {
                match self {
                    #(#to_value_arms),*
                }
            }

            pub fn from_key_value(key: &str, value: loro::LoroValue) -> Option<Self> {
                match key {
                    #(#from_value_arms,)*
                    _ => None,
                }
            }
        }
    };

    TokenStream::from(expanded)
}
