use editor_bindgen::meta::{
    FfiInterface, FfiMethod, FfiParam, FfiParamType, FfiReturnType, FfiScalarParam, FfiScalarReturn,
};
use syn::ItemImpl;

pub fn extract(item: &ItemImpl) -> FfiInterface {
    let impl_name = extract_impl_name(item);

    let methods = item
        .items
        .iter()
        .filter_map(|item| {
            if let syn::ImplItem::Fn(method) = item {
                Some(extract_method(method, &impl_name))
            } else {
                None
            }
        })
        .collect();

    FfiInterface {
        name: impl_name,
        methods,
    }
}

fn extract_impl_name(item: &ItemImpl) -> String {
    if let syn::Type::Path(type_path) = item.self_ty.as_ref()
        && let Some(seg) = type_path.path.segments.last()
    {
        return seg.ident.to_string();
    }
    panic!("could not extract impl type name");
}

fn extract_method(method: &syn::ImplItemFn, impl_name: &str) -> FfiMethod {
    let name = method.sig.ident.to_string();
    let is_async = method.sig.asyncness.is_some();
    let is_constructor = has_constructor_attr(&method.attrs);

    let params = method
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                let param_name = extract_param_name(&pat_type.pat);
                let ty = parse_param_type(&pat_type.ty);
                Some(FfiParam {
                    name: param_name,
                    ty,
                })
            } else {
                // Skip &self
                None
            }
        })
        .collect();

    let return_type = parse_return_type(&method.sig.output, impl_name);

    FfiMethod {
        name,
        is_async,
        is_constructor,
        params,
        return_type,
    }
}

fn extract_param_name(pat: &syn::Pat) -> String {
    if let syn::Pat::Ident(ident) = pat {
        ident.ident.to_string()
    } else {
        "_".into()
    }
}

fn has_constructor_attr(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        // Direct: #[uniffi::constructor]
        if path_matches(attr.path(), &["uniffi", "constructor"]) {
            return true;
        }
        // Via cfg_attr: #[cfg_attr(predicate, uniffi::constructor)]
        if attr.path().is_ident("cfg_attr")
            && let syn::Meta::List(list) = &attr.meta
        {
            let result = list.parse_args_with(|input: syn::parse::ParseStream| {
                let _predicate: syn::Meta = input.parse()?;
                let _comma: syn::Token![,] = input.parse()?;
                let path: syn::Path = input.parse()?;
                Ok(path)
            });
            if let Ok(path) = result
                && path_matches(&path, &["uniffi", "constructor"])
            {
                return true;
            }
        }
    }
    false
}

fn path_matches(path: &syn::Path, segments: &[&str]) -> bool {
    if path.segments.len() != segments.len() {
        return false;
    }
    path.segments
        .iter()
        .zip(segments.iter())
        .all(|(seg, expected)| seg.ident == expected)
}

fn type_name(ty: &syn::Type) -> Option<String> {
    if let syn::Type::Path(type_path) = ty {
        let seg = type_path.path.segments.last()?;
        let ident = seg.ident.to_string();
        if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
            let mapped = args
                .args
                .iter()
                .filter_map(|arg| match arg {
                    syn::GenericArgument::Type(ty) => type_name(ty),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if !mapped.is_empty() {
                return Some(format!("{}<{}>", ident, mapped.join(", ")));
            }
        }
        return Some(ident);
    }

    if let syn::Type::Tuple(tuple) = ty
        && tuple.elems.is_empty()
    {
        return Some("()".into());
    }

    None
}

fn extract_angle_bracketed_inner(seg: &syn::PathSegment) -> Option<&syn::Type> {
    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments
        && args.args.len() == 1
        && let syn::GenericArgument::Type(inner) = args.args.first()?
    {
        return Some(inner);
    }
    None
}

fn parse_param_type(ty: &syn::Type) -> FfiParamType {
    if let syn::Type::Path(type_path) = ty
        && let Some(seg) = type_path.path.segments.last()
    {
        let ident = seg.ident.to_string();

        match ident.as_str() {
            "Complex" => {
                if let Some(inner) = extract_angle_bracketed_inner(seg) {
                    let inner_name = type_name(inner).unwrap_or_default();
                    return FfiParamType::Complex(inner_name);
                }
            }
            "Vec" => {
                if let Some(inner) = extract_angle_bracketed_inner(seg) {
                    return FfiParamType::Vec(parse_scalar_param(inner));
                }
            }
            "Option" => {
                if let Some(inner) = extract_angle_bracketed_inner(seg) {
                    return FfiParamType::Option(parse_scalar_param(inner));
                }
            }
            _ => {}
        }

        return FfiParamType::Primitive(ident);
    }
    FfiParamType::Primitive("unknown".into())
}

fn parse_scalar_param(ty: &syn::Type) -> FfiScalarParam {
    if let syn::Type::Path(type_path) = ty
        && let Some(seg) = type_path.path.segments.last()
    {
        let ident = seg.ident.to_string();
        if ident == "Complex"
            && let Some(inner) = extract_angle_bracketed_inner(seg)
        {
            let inner_name = type_name(inner).unwrap_or_default();
            return FfiScalarParam::Complex(inner_name);
        }
        return FfiScalarParam::Primitive(ident);
    }
    FfiScalarParam::Primitive("unknown".into())
}

fn parse_return_type(output: &syn::ReturnType, impl_name: &str) -> FfiReturnType {
    match output {
        syn::ReturnType::Default => FfiReturnType::Unit,
        syn::ReturnType::Type(_, ty) => parse_return_type_inner(ty, impl_name),
    }
}

fn parse_return_type_inner(ty: &syn::Type, impl_name: &str) -> FfiReturnType {
    if let syn::Type::Path(type_path) = ty
        && let Some(seg) = type_path.path.segments.last()
    {
        let ident = seg.ident.to_string();

        match ident.as_str() {
            "EditorResult" => {
                if let Some(inner) = extract_angle_bracketed_inner(seg) {
                    return parse_return_type_inner(inner, impl_name);
                }
            }
            "Owned" => {
                if let Some(inner) = extract_angle_bracketed_inner(seg) {
                    let inner_name = resolve_self(type_name(inner), impl_name);
                    return FfiReturnType::Owned(inner_name);
                }
            }
            "Complex" => {
                if let Some(inner) = extract_angle_bracketed_inner(seg) {
                    let inner_name = type_name(inner).unwrap_or_default();
                    return FfiReturnType::Complex(inner_name);
                }
            }
            "Vec" => {
                if let Some(inner) = extract_angle_bracketed_inner(seg) {
                    return FfiReturnType::Vec(parse_scalar_return(inner, impl_name));
                }
            }
            "Option" => {
                if let Some(inner) = extract_angle_bracketed_inner(seg) {
                    return FfiReturnType::Option(parse_scalar_return(inner, impl_name));
                }
            }
            _ => return FfiReturnType::Primitive(ident),
        }
    }

    // Handle unit tuple type `()`
    if let syn::Type::Tuple(tuple) = ty
        && tuple.elems.is_empty()
    {
        return FfiReturnType::Unit;
    }

    FfiReturnType::Primitive("unknown".into())
}

fn parse_scalar_return(ty: &syn::Type, impl_name: &str) -> FfiScalarReturn {
    if let syn::Type::Path(type_path) = ty
        && let Some(seg) = type_path.path.segments.last()
    {
        let ident = seg.ident.to_string();
        match ident.as_str() {
            "Complex" => {
                if let Some(inner) = extract_angle_bracketed_inner(seg) {
                    let inner_name = type_name(inner).unwrap_or_default();
                    return FfiScalarReturn::Complex(inner_name);
                }
            }
            "Owned" => {
                if let Some(inner) = extract_angle_bracketed_inner(seg) {
                    let inner_name = resolve_self(type_name(inner), impl_name);
                    return FfiScalarReturn::Owned(inner_name);
                }
            }
            _ => return FfiScalarReturn::Primitive(ident),
        }
    }
    FfiScalarReturn::Primitive("unknown".into())
}

fn resolve_self(name: Option<String>, impl_name: &str) -> String {
    match name.as_deref() {
        Some("Self") => impl_name.into(),
        Some(n) => n.into(),
        None => impl_name.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complex_return_preserves_generic_type_arguments() {
        let method: syn::ImplItemFn = syn::parse_quote! {
            pub fn applied_style(
                &self,
            ) -> EditorResult<Complex<editor_common::Tri<editor_core::StyleRefValue>>> {
                todo!()
            }
        };

        let extracted = extract_method(&method, "Editor");

        assert_eq!(
            extracted.return_type,
            FfiReturnType::Complex("Tri<StyleRefValue>".into())
        );
    }

    #[test]
    fn complex_param_preserves_generic_type_arguments() {
        let ty: syn::Type = syn::parse_quote! {
            Complex<editor_common::Tri<editor_core::StyleRefValue>>
        };

        assert_eq!(
            parse_param_type(&ty),
            FfiParamType::Complex("Tri<StyleRefValue>".into())
        );
    }
}
