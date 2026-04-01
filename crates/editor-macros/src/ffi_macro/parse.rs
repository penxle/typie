use syn::parse::ParseStream;
use syn::{Data, DeriveInput, Fields, Ident, Type, Variant, parenthesized};

pub struct FfiInput {
    pub item: DeriveInput,
    pub custom: Option<Type>,
}

impl FfiInput {
    pub fn from_attr_and_item(attr: proc_macro2::TokenStream, item: DeriveInput) -> Self {
        if attr.is_empty() {
            return Self { item, custom: None };
        }

        let parsed: FfiAttr = syn::parse2(attr).expect("expected `custom` or `custom(TargetType)`");
        Self {
            item,
            custom: Some(parsed.target),
        }
    }
}

struct FfiAttr {
    target: Type,
}

impl syn::parse::Parse for FfiAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        if ident != "custom" {
            return Err(syn::Error::new(ident.span(), "expected `custom`"));
        }

        let content;
        parenthesized!(content in input);
        let target: Type = content.parse()?;

        Ok(FfiAttr { target })
    }
}

pub enum FfiTypeKind {
    Custom { target: Type },
    Struct { fields: Vec<StructField> },
    Enum { variants: Vec<EnumVariant> },
}

pub struct StructField {
    pub name: Ident,
    pub ty: Type,
}

pub enum EnumVariant {
    Unit {
        name: Ident,
    },
    Tuple {
        name: Ident,
        fields: Vec<Type>,
    },
    Struct {
        name: Ident,
        fields: Vec<StructField>,
    },
}

impl FfiInput {
    pub fn kind(&self) -> FfiTypeKind {
        if let Some(target) = &self.custom {
            return FfiTypeKind::Custom {
                target: target.clone(),
            };
        }
        match &self.item.data {
            Data::Struct(data) => {
                let fields = match &data.fields {
                    Fields::Named(named) => named
                        .named
                        .iter()
                        .map(|f| StructField {
                            name: f.ident.clone().unwrap(),
                            ty: f.ty.clone(),
                        })
                        .collect(),
                    _ => panic!("#[ffi] structs must have named fields"),
                };
                FfiTypeKind::Struct { fields }
            }
            Data::Enum(data) => {
                let variants = data.variants.iter().map(|v| parse_variant(v)).collect();
                FfiTypeKind::Enum { variants }
            }
            Data::Union(_) => panic!("#[ffi] does not support unions"),
        }
    }
}

fn parse_variant(v: &Variant) -> EnumVariant {
    let name = v.ident.clone();
    match &v.fields {
        Fields::Unit => EnumVariant::Unit { name },
        Fields::Unnamed(fields) => {
            let types = fields.unnamed.iter().map(|f| f.ty.clone()).collect();
            EnumVariant::Tuple {
                name,
                fields: types,
            }
        }
        Fields::Named(fields) => {
            let parsed = fields
                .named
                .iter()
                .map(|f| StructField {
                    name: f.ident.clone().unwrap(),
                    ty: f.ty.clone(),
                })
                .collect();
            EnumVariant::Struct {
                name,
                fields: parsed,
            }
        }
    }
}
