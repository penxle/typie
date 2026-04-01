use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Path, Token, Type, braced, parenthesized};

pub enum FfiGenInput {
    CustomType {
        source: Path,
        target: Type,
    },
    Struct {
        name: Ident,
        source: Path,
        fields: Vec<StructField>,
    },
    Enum {
        name: Ident,
        source: Path,
        variants: Vec<EnumVariant>,
    },
}

pub struct StructField {
    pub name: Ident,
    pub ty: Type,
}

pub enum EnumVariant {
    Unit {
        name: Ident,
        source_path: Path,
    },
    Tuple {
        name: Ident,
        bindings: Vec<(Ident, Type)>,
        source_path: Path,
    },
    Named {
        name: Ident,
        fields: Vec<StructField>,
        source_path: Path,
    },
}

impl Parse for FfiGenInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if !input.peek(Token![@]) {
            return Err(input.error("expected @custom_type, @struct, or @enum"));
        }
        input.parse::<Token![@]>()?;

        let tag = Ident::parse_any(input)?;
        match tag.to_string().as_str() {
            "custom_type" => parse_custom_type(input),
            "struct" => parse_struct(input),
            "enum" => parse_enum(input),
            _ => Err(syn::Error::new(
                tag.span(),
                "expected custom_type, struct, or enum",
            )),
        }
    }
}

fn parse_name_and_source(input: ParseStream) -> syn::Result<(Ident, Path)> {
    let name: Ident = input.parse()?;
    input.parse::<Token![=]>()?;
    let source: Path = input.parse()?;
    input.parse::<Token![;]>()?;
    Ok((name, source))
}

fn parse_custom_type(input: ParseStream) -> syn::Result<FfiGenInput> {
    let name: Ident = input.parse()?;
    input.parse::<Token![=]>()?;
    let source: Path = input.parse()?;
    input.parse::<Token![:]>()?;
    let target: Type = input.parse()?;
    input.parse::<Token![;]>()?;
    let _ = name;
    Ok(FfiGenInput::CustomType { source, target })
}

fn parse_struct(input: ParseStream) -> syn::Result<FfiGenInput> {
    let (name, source) = parse_name_and_source(input)?;
    let mut fields = Vec::new();

    while input.peek(Token![@]) {
        let fork = input.fork();
        fork.parse::<Token![@]>()?;
        let tag: Ident = fork.parse()?;
        if tag == "end" {
            input.parse::<Token![@]>()?;
            let _: Ident = input.parse()?;
            input.parse::<Token![;]>()?;
            break;
        }
        input.parse::<Token![@]>()?;
        let _field_tag: Ident = input.parse()?;
        let field_name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let field_ty: Type = input.parse()?;
        input.parse::<Token![;]>()?;
        fields.push(StructField {
            name: field_name,
            ty: field_ty,
        });
    }

    Ok(FfiGenInput::Struct {
        name,
        source,
        fields,
    })
}

fn parse_enum(input: ParseStream) -> syn::Result<FfiGenInput> {
    let (name, source) = parse_name_and_source(input)?;
    let mut variants = Vec::new();

    while input.peek(Token![@]) {
        let fork = input.fork();
        fork.parse::<Token![@]>()?;
        let tag: Ident = fork.parse()?;

        if tag == "end" {
            input.parse::<Token![@]>()?;
            let _: Ident = input.parse()?;
            input.parse::<Token![;]>()?;
            break;
        }

        input.parse::<Token![@]>()?;
        let variant_tag: Ident = input.parse()?;

        match variant_tag.to_string().as_str() {
            "unit" => {
                let variant_name: Ident = input.parse()?;
                input.parse::<Token![=]>()?;
                let source_path: Path = input.parse()?;
                input.parse::<Token![;]>()?;
                variants.push(EnumVariant::Unit {
                    name: variant_name,
                    source_path,
                });
            }
            "tuple" => {
                let variant_name: Ident = input.parse()?;
                let content;
                parenthesized!(content in input);
                let mut bindings = Vec::new();
                while !content.is_empty() {
                    let var: Ident = content.parse()?;
                    content.parse::<Token![:]>()?;
                    let ty: Type = content.parse()?;
                    bindings.push((var, ty));
                    if !content.is_empty() {
                        content.parse::<Token![,]>()?;
                    }
                }
                input.parse::<Token![=]>()?;
                let source_path: Path = input.parse()?;
                input.parse::<Token![;]>()?;
                variants.push(EnumVariant::Tuple {
                    name: variant_name,
                    bindings,
                    source_path,
                });
            }
            "named" => {
                let variant_name: Ident = input.parse()?;
                let content;
                braced!(content in input);
                let mut fields = Vec::new();
                while !content.is_empty() {
                    let fname: Ident = content.parse()?;
                    content.parse::<Token![:]>()?;
                    let fty: Type = content.parse()?;
                    fields.push(StructField {
                        name: fname,
                        ty: fty,
                    });
                    if !content.is_empty() {
                        content.parse::<Token![,]>()?;
                    }
                }
                input.parse::<Token![=]>()?;
                let source_path: Path = input.parse()?;
                input.parse::<Token![;]>()?;
                variants.push(EnumVariant::Named {
                    name: variant_name,
                    fields,
                    source_path,
                });
            }
            other => {
                return Err(syn::Error::new(
                    variant_tag.span(),
                    format!("unknown variant tag: {}", other),
                ));
            }
        }
    }

    Ok(FfiGenInput::Enum {
        name,
        source,
        variants,
    })
}
