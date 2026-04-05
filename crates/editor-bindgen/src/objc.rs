use heck::{ToLowerCamelCase, ToUpperCamelCase};

use crate::meta::{FfiMethod, FfiParam};

/// Prepositions that cause Swift to omit the `with` prefix when generating
/// Objective-C selectors. Source: swift/lib/Basic/PartsOfSpeech.def
const PREPOSITIONS: &[&str] = &[
    "above",
    "after",
    "along",
    "alongside",
    "as",
    "at",
    "before",
    "below",
    "by",
    "following",
    "for",
    "from",
    "given",
    "in",
    "including",
    "inside",
    "into",
    "matching",
    "of",
    "on",
    "passing",
    "preceding",
    "since",
    "to",
    "until",
    "using",
    "via",
    "when",
    "with",
    "within",
];

/// Checks whether a camelCase name's leading lowercase word is a preposition.
fn first_word_is_preposition(camel: &str) -> bool {
    let first_word_end = camel
        .char_indices()
        .find(|(_, c)| c.is_ascii_uppercase())
        .map(|(i, _)| i)
        .unwrap_or(camel.len());
    let first_word = &camel[..first_word_end];
    PREPOSITIONS.contains(&first_word)
}

/// Checks whether a camelCase name's trailing word (last uppercase-started
/// segment, or the whole string if no uppercase letters) is a preposition.
fn last_word_is_preposition(camel: &str) -> bool {
    let last_word_start = camel
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_ascii_uppercase())
        .map(|(i, _)| i)
        .unwrap_or(0);
    let last_word = &camel[last_word_start..].to_ascii_lowercase();
    PREPOSITIONS.contains(&last_word.as_str())
}

/// Swift's ObjC selector rule: the `With` prefix before the first argument
/// is omitted when EITHER the base method name ends with a preposition OR
/// the first argument name starts with a preposition.
/// Source: swift/lib/AST/Decl.cpp (AbstractFunctionDecl::getObjCSelector).
pub fn objc_selector(method: &FfiMethod) -> String {
    let base = method.name.to_lower_camel_case();
    if method.params.is_empty() {
        format!("{}WithError", base)
    } else {
        let first_camel = method.params[0].name.to_lower_camel_case();
        let first_upper = method.params[0].name.to_upper_camel_case();
        let drop_with = first_word_is_preposition(&first_camel) || last_word_is_preposition(&base);
        if drop_with {
            format!("{}{}", base, first_upper)
        } else {
            format!("{}With{}", base, first_upper)
        }
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
    fn selector_preposition_drops_with() {
        let m = method(
            "input_context",
            vec![
                param("before_limit", FfiParamType::Primitive("u32".into())),
                param("after_limit", FfiParamType::Primitive("u32".into())),
            ],
        );
        assert_eq!(objc_selector(&m), "inputContextBeforeLimit");
    }

    #[test]
    fn selector_single_preposition_word_drops_with() {
        // First param is a single preposition — treated as starting with itself.
        let m = method(
            "take_snapshot",
            vec![param("at", FfiParamType::Primitive("u32".into()))],
        );
        assert_eq!(objc_selector(&m), "takeSnapshotAt");
    }

    #[test]
    fn selector_base_ending_preposition_drops_with() {
        // `move_to` → `moveTo`; base ends with preposition "to".
        let m = method(
            "move_to",
            vec![param("point", FfiParamType::Primitive("u32".into()))],
        );
        assert_eq!(objc_selector(&m), "moveToPoint");
    }

    #[test]
    fn selector_base_ending_at_drops_with() {
        let m = method(
            "insert_at",
            vec![param("index", FfiParamType::Primitive("u32".into()))],
        );
        assert_eq!(objc_selector(&m), "insertAtIndex");
    }

    #[test]
    fn selector_base_ending_from_drops_with() {
        let m = method(
            "fetch_from",
            vec![param("url", FfiParamType::Primitive("String".into()))],
        );
        assert_eq!(objc_selector(&m), "fetchFromUrl");
    }

    #[test]
    fn selector_single_word_base_no_preposition() {
        // `move` alone is not a preposition; param doesn't start with one.
        let m = method(
            "move",
            vec![param("point", FfiParamType::Primitive("u32".into()))],
        );
        assert_eq!(objc_selector(&m), "moveWithPoint");
    }

    #[test]
    fn selector_non_preposition_keeps_with() {
        // `message` does not start with a preposition.
        let m = method(
            "enqueue",
            vec![param("message", FfiParamType::Primitive("String".into()))],
        );
        assert_eq!(objc_selector(&m), "enqueueWithMessage");
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
