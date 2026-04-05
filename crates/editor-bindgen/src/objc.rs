use heck::{ToLowerCamelCase, ToUpperCamelCase};

use crate::meta::{FfiMethod, FfiParam};

pub fn objc_selector(method: &FfiMethod) -> String {
    let base = method.name.to_lower_camel_case();
    if method.params.is_empty() {
        format!("{}WithError", base)
    } else {
        let first = method.params[0].name.to_upper_camel_case();
        format!("{}With{}", base, first)
    }
}

pub fn kotlin_cinterop_args(
    params: &[FfiParam],
    format_value: impl Fn(&FfiParam) -> String,
) -> String {
    if params.is_empty() {
        return String::new();
    }
    let mut parts = Vec::new();
    for (i, param) in params.iter().enumerate() {
        let value = format_value(param);
        if i == 0 {
            parts.push(value);
        } else {
            let label = param.name.to_lower_camel_case();
            parts.push(format!("{} = {}", label, value));
        }
    }
    parts.join(", ")
}

pub fn swift_param_decl(params: &[FfiParam], format_type: impl Fn(&FfiParam) -> String) -> String {
    params
        .iter()
        .map(|p| {
            let name = p.name.to_lower_camel_case();
            let ty = format_type(p);
            format!("{}: {}", name, ty)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn swift_call_args(params: &[FfiParam], format_value: impl Fn(&FfiParam) -> String) -> String {
    params
        .iter()
        .map(|p| {
            let name = p.name.to_lower_camel_case();
            let value = format_value(p);
            format!("{}: {}", name, value)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::{FfiParamType, FfiReturnType};

    fn param(name: &str, ty: FfiParamType) -> FfiParam {
        FfiParam {
            name: name.into(),
            ty,
        }
    }

    fn method(name: &str, params: Vec<FfiParam>) -> FfiMethod {
        FfiMethod {
            name: name.into(),
            is_async: false,
            is_constructor: false,
            params,
            return_type: FfiReturnType::Unit,
        }
    }

    #[test]
    fn selector_no_params() {
        let m = method("backend_kind", vec![]);
        assert_eq!(objc_selector(&m), "backendKindWithError");
    }

    #[test]
    fn selector_with_params() {
        let m = method(
            "create_editor",
            vec![
                param("doc", FfiParamType::Complex("Doc".into())),
                param("selection", FfiParamType::Complex("Selection".into())),
            ],
        );
        assert_eq!(objc_selector(&m), "createEditorWithDoc");
    }

    #[test]
    fn selector_single_param() {
        let m = method(
            "load_icu_data",
            vec![param(
                "data",
                FfiParamType::Vec(crate::meta::FfiScalarParam::Primitive("u8".into())),
            )],
        );
        assert_eq!(objc_selector(&m), "loadIcuDataWithData");
    }

    #[test]
    fn selector_font_method() {
        let m = method(
            "load_font_base",
            vec![
                param("family", FfiParamType::Primitive("String".into())),
                param("weight", FfiParamType::Primitive("u16".into())),
                param(
                    "data",
                    FfiParamType::Vec(crate::meta::FfiScalarParam::Primitive("u8".into())),
                ),
            ],
        );
        assert_eq!(objc_selector(&m), "loadFontBaseWithFamily");
    }

    #[test]
    fn kotlin_cinterop_args_formatting() {
        let params = vec![
            param("family", FfiParamType::Primitive("String".into())),
            param("weight", FfiParamType::Primitive("u16".into())),
            param(
                "data",
                FfiParamType::Vec(crate::meta::FfiScalarParam::Primitive("u8".into())),
            ),
        ];
        let result = kotlin_cinterop_args(&params, |p| p.name.to_lower_camel_case());
        assert_eq!(result, "family, weight = weight, data = data");
    }

    #[test]
    fn swift_param_decl_formatting() {
        let params = vec![
            param("family", FfiParamType::Primitive("String".into())),
            param("weight", FfiParamType::Primitive("u16".into())),
        ];
        let result = swift_param_decl(&params, |_| "String".into());
        assert_eq!(result, "family: String, weight: String");
    }
}
