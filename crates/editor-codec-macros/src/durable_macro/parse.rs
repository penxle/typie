use syn::{Data, DeriveInput, Expr, Fields, Ident, LitInt, LitStr, Type};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    Evolvable,
    Frozen,
    Open,
    Closed,
}

pub struct Input {
    pub ident: Ident,
    pub grade: Grade,
    pub retired: Vec<u64>,
    pub kind: Kind,
}

pub enum Kind {
    Struct(StructKind),
    Enum(EnumKind),
}

pub struct StructKind {
    pub fields: Vec<Field>,
    pub tail: Option<Ident>,
}

pub struct EnumKind {
    pub variants: Vec<Variant>,
    pub unknown: Option<Ident>,
}

pub struct Field {
    pub ident: Option<Ident>,
    pub ty: Type,
    pub default: Option<Option<(Expr, String)>>,
}

pub struct Variant {
    pub ident: Ident,
    pub tag: u64,
    pub frozen_payload: bool,
    pub named: bool,
    pub fields: Vec<Field>,
    pub tail: Option<Ident>,
}

fn is_unknown_tail(ty: &Type) -> bool {
    matches!(ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "UnknownTail"))
}

fn is_unknown_payload(ty: &Type) -> bool {
    matches!(ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "UnknownPayload"))
}

struct TypeAttrs {
    grade: Option<Grade>,
    retired: Vec<u64>,
}

fn parse_type_attrs(attrs: &[syn::Attribute]) -> syn::Result<TypeAttrs> {
    let mut out = TypeAttrs {
        grade: None,
        retired: Vec::new(),
    };
    for attr in attrs {
        if !attr.path().is_ident("durable") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            let set_grade = |out: &mut TypeAttrs, grade: Grade| -> syn::Result<()> {
                if out.grade.is_some() {
                    return Err(meta.error("duplicate durable grade"));
                }
                out.grade = Some(grade);
                Ok(())
            };
            if meta.path.is_ident("evolvable") {
                set_grade(&mut out, Grade::Evolvable)
            } else if meta.path.is_ident("frozen") {
                set_grade(&mut out, Grade::Frozen)
            } else if meta.path.is_ident("open") {
                set_grade(&mut out, Grade::Open)
            } else if meta.path.is_ident("closed") {
                set_grade(&mut out, Grade::Closed)
            } else if meta.path.is_ident("retired") {
                let content;
                syn::parenthesized!(content in meta.input);
                let tags = content.parse_terminated(|p| p.parse::<LitInt>(), syn::Token![,])?;
                for tag in tags {
                    let tag: u64 = tag.base10_parse()?;
                    if out.retired.contains(&tag) {
                        return Err(meta.error(format!("duplicate retired tag {tag}")));
                    }
                    out.retired.push(tag);
                }
                Ok(())
            } else {
                Err(meta.error("unknown durable attribute"))
            }
        })?;
    }
    Ok(out)
}

fn parse_field(field: &syn::Field) -> syn::Result<Field> {
    let mut default: Option<Option<(Expr, String)>> = None;
    for attr in &field.attrs {
        if !attr.path().is_ident("durable") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                if default.is_some() {
                    return Err(meta.error("duplicate durable default"));
                }
                if meta.input.peek(syn::Token![=]) {
                    let value = meta.value()?;
                    let lit: LitStr = value.parse()?;
                    let raw = lit.value();
                    default = Some(Some((lit.parse::<Expr>()?, raw)));
                } else {
                    default = Some(None);
                }
                Ok(())
            } else {
                Err(meta.error("unknown durable attribute on field"))
            }
        })?;
    }
    Ok(Field {
        ident: field.ident.clone(),
        ty: field.ty.clone(),
        default,
    })
}

fn parse_fields(
    ident: &Ident,
    fields: &Fields,
    payload_frozen: bool,
    tail_error: &str,
) -> syn::Result<(Vec<Field>, Option<Ident>, bool)> {
    let (named, raw): (bool, Vec<&syn::Field>) = match fields {
        Fields::Named(f) => (true, f.named.iter().collect()),
        Fields::Unnamed(f) => (false, f.unnamed.iter().collect()),
        Fields::Unit => (true, Vec::new()),
    };
    let mut parsed: Vec<Field> = raw
        .iter()
        .map(|f| parse_field(f))
        .collect::<syn::Result<_>>()?;

    if payload_frozen {
        if parsed.iter().any(|f| is_unknown_tail(&f.ty)) {
            return Err(syn::Error::new_spanned(
                ident,
                "frozen payloads must not contain UnknownTail",
            ));
        }
        return Ok((parsed, None, named));
    }

    let Some(last) = parsed.last() else {
        return Err(syn::Error::new_spanned(ident, tail_error));
    };
    if !is_unknown_tail(&last.ty) {
        return Err(syn::Error::new_spanned(ident, tail_error));
    }
    let tail = parsed.pop().expect("checked non-empty");
    if parsed.iter().any(|f| is_unknown_tail(&f.ty)) {
        return Err(syn::Error::new_spanned(
            ident,
            "UnknownTail must be the last field only",
        ));
    }
    if tail.default.is_some() {
        return Err(syn::Error::new_spanned(
            ident,
            "UnknownTail field cannot have durable attributes",
        ));
    }
    let tail_ident = tail
        .ident
        .clone()
        .ok_or_else(|| syn::Error::new_spanned(ident, "UnknownTail field must be a named field"))?;
    Ok((parsed, Some(tail_ident), named))
}

pub fn parse(input: &DeriveInput) -> syn::Result<Input> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "Durable does not support generic parameters",
        ));
    }
    let attrs = parse_type_attrs(&input.attrs)?;

    match &input.data {
        Data::Struct(data) => {
            let grade = match attrs.grade {
                Some(Grade::Evolvable) => Grade::Evolvable,
                Some(Grade::Frozen) => Grade::Frozen,
                Some(_) => {
                    return Err(syn::Error::new_spanned(
                        &input.ident,
                        "open/closed grades are enum-only",
                    ));
                }
                None => {
                    return Err(syn::Error::new_spanned(
                        &input.ident,
                        "missing #[durable(...)] grade (struct: evolvable|frozen, enum: open|closed)",
                    ));
                }
            };
            if !attrs.retired.is_empty() {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "retired is only allowed on open enums",
                ));
            }
            if !matches!(&data.fields, Fields::Named(_)) {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "Durable structs must use named fields",
                ));
            }
            let frozen = matches!(grade, Grade::Frozen);
            let (fields, tail, _named) = parse_fields(
                &input.ident,
                &data.fields,
                frozen,
                "evolvable struct requires an UnknownTail as its last field",
            )?;
            for field in &fields {
                if field.default.is_some() && frozen {
                    return Err(syn::Error::new_spanned(
                        &input.ident,
                        "frozen struct fields cannot be defaulted",
                    ));
                }
            }
            Ok(Input {
                ident: input.ident.clone(),
                grade,
                retired: Vec::new(),
                kind: Kind::Struct(StructKind { fields, tail }),
            })
        }
        Data::Enum(data) => {
            let grade = match attrs.grade {
                Some(Grade::Open) => Grade::Open,
                Some(Grade::Closed) => Grade::Closed,
                Some(_) => {
                    return Err(syn::Error::new_spanned(
                        &input.ident,
                        "evolvable/frozen grades are struct-only",
                    ));
                }
                None => {
                    return Err(syn::Error::new_spanned(
                        &input.ident,
                        "missing #[durable(...)] grade (struct: evolvable|frozen, enum: open|closed)",
                    ));
                }
            };
            let open = matches!(grade, Grade::Open);
            if !open && !attrs.retired.is_empty() {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "retired is only allowed on open enums",
                ));
            }

            let mut variants = Vec::new();
            let mut unknown: Option<Ident> = None;
            let mut seen_tags: Vec<u64> = Vec::new();

            for variant in &data.variants {
                let mut tag: Option<u64> = None;
                let mut frozen_payload = false;
                let mut is_unknown = false;
                for attr in &variant.attrs {
                    if !attr.path().is_ident("durable") {
                        continue;
                    }
                    attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("n") {
                            if tag.is_some() {
                                return Err(meta.error("duplicate tag attribute"));
                            }
                            let content;
                            syn::parenthesized!(content in meta.input);
                            let lit: LitInt = content.parse()?;
                            if !content.is_empty() {
                                return Err(meta.error("n(...) takes exactly one tag"));
                            }
                            tag = Some(lit.base10_parse()?);
                            Ok(())
                        } else if meta.path.is_ident("frozen") {
                            frozen_payload = true;
                            Ok(())
                        } else if meta.path.is_ident("unknown") {
                            is_unknown = true;
                            Ok(())
                        } else {
                            Err(meta.error("unknown durable attribute on variant"))
                        }
                    })?;
                }

                if is_unknown {
                    if !open {
                        return Err(syn::Error::new_spanned(
                            variant,
                            "closed enum must not have an unknown variant",
                        ));
                    }
                    if tag.is_some() {
                        return Err(syn::Error::new_spanned(
                            variant,
                            "unknown variant must not have a tag",
                        ));
                    }
                    if unknown.is_some() {
                        return Err(syn::Error::new_spanned(
                            variant,
                            "duplicate #[durable(unknown)] variant",
                        ));
                    }
                    let ok = matches!(&variant.fields, Fields::Unnamed(f)
                        if f.unnamed.len() == 1 && is_unknown_payload(&f.unnamed[0].ty));
                    if !ok {
                        return Err(syn::Error::new_spanned(
                            variant,
                            "#[durable(unknown)] variant must have exactly one UnknownPayload field",
                        ));
                    }
                    unknown = Some(variant.ident.clone());
                    continue;
                }

                let Some(tag) = tag else {
                    return Err(syn::Error::new_spanned(
                        variant,
                        "missing #[durable(n(N))] on variant",
                    ));
                };
                if seen_tags.contains(&tag) {
                    return Err(syn::Error::new_spanned(
                        variant,
                        format!("duplicate tag {tag}"),
                    ));
                }
                if attrs.retired.contains(&tag) {
                    return Err(syn::Error::new_spanned(
                        variant,
                        format!("tag {tag} is retired and must not be reused"),
                    ));
                }
                seen_tags.push(tag);

                if !open {
                    if !matches!(&variant.fields, Fields::Unit) {
                        return Err(syn::Error::new_spanned(
                            variant,
                            "closed enum variants must be unit",
                        ));
                    }
                    variants.push(Variant {
                        ident: variant.ident.clone(),
                        tag,
                        frozen_payload: true,
                        named: true,
                        fields: Vec::new(),
                        tail: None,
                    });
                    continue;
                }

                let unit = matches!(&variant.fields, Fields::Unit);
                let tuple = matches!(&variant.fields, Fields::Unnamed(_));
                let effective_frozen = frozen_payload || unit;
                if tuple && !effective_frozen {
                    return Err(syn::Error::new_spanned(
                        variant,
                        "tuple variant payloads must be #[durable(frozen)]",
                    ));
                }
                let (fields, tail, named) = parse_fields(
                    &variant.ident,
                    &variant.fields,
                    effective_frozen,
                    "evolvable variant payload requires an UnknownTail as its last field (or mark the variant #[durable(frozen)])",
                )?;
                for field in &fields {
                    if field.default.is_some() && effective_frozen {
                        return Err(syn::Error::new_spanned(
                            variant,
                            "frozen payload fields cannot be defaulted",
                        ));
                    }
                }
                variants.push(Variant {
                    ident: variant.ident.clone(),
                    tag,
                    frozen_payload: effective_frozen,
                    named,
                    fields,
                    tail,
                });
            }

            if open && unknown.is_none() {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "open enum requires exactly one #[durable(unknown)] variant",
                ));
            }

            Ok(Input {
                ident: input.ident.clone(),
                grade,
                retired: attrs.retired,
                kind: Kind::Enum(EnumKind { variants, unknown }),
            })
        }
        Data::Union(_) => Err(syn::Error::new_spanned(
            &input.ident,
            "Durable does not support unions",
        )),
    }
}
