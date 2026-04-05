use std::collections::HashMap;
use std::path::Path;

use heck::ToLowerCamelCase;

use crate::meta::{
    FfiInterface, FfiMethod, FfiParamType, FfiReturnType, FfiScalarParam, FfiScalarReturn,
};

const PACKAGE: &str = "co.typie.editor.ffi";

pub fn generate_all(
    interfaces: &[FfiInterface],
    custom_types: &HashMap<String, String>,
    output_dir: &Path,
) {
    let pkg_dir = output_dir.join(PACKAGE.replace('.', "/"));
    std::fs::create_dir_all(&pkg_dir).expect("failed to create output directory");

    for iface in interfaces {
        let content = generate_interface(iface, custom_types);
        let path = pkg_dir.join(format!("{}.kt", iface.name));
        std::fs::write(&path, content).expect("failed to write file");
    }
}

pub fn generate_interface(iface: &FfiInterface, custom_types: &HashMap<String, String>) -> String {
    let mut out = String::new();
    out.push_str(&format!("package {}\n\n", PACKAGE));
    out.push_str(&format!("interface {} {{\n", iface.name));
    for method in &iface.methods {
        if method.is_constructor {
            continue;
        }
        out.push_str(&format!(
            "    {}\n",
            format_method_sig(method, custom_types)
        ));
    }
    out.push_str("}\n");
    out
}

fn format_method_sig(method: &FfiMethod, custom_types: &HashMap<String, String>) -> String {
    let kt_name = method.name.to_lower_camel_case();
    let params = method
        .params
        .iter()
        .map(|p| {
            format!(
                "{}: {}",
                p.name.to_lower_camel_case(),
                param_to_kotlin(&p.ty, custom_types)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let ret = return_to_kotlin(&method.return_type, custom_types);
    if ret.is_empty() {
        format!("fun {}({})", kt_name, params)
    } else {
        format!("fun {}({}): {}", kt_name, params, ret)
    }
}

pub fn param_to_kotlin(ty: &FfiParamType, custom_types: &HashMap<String, String>) -> String {
    match ty {
        FfiParamType::Primitive(p) => resolve_primitive(p, custom_types),
        FfiParamType::Complex(name) => name.clone(),
        FfiParamType::Vec(inner) => {
            if matches!(inner, FfiScalarParam::Primitive(p) if p == "u8") {
                "ByteArray".into()
            } else {
                format!("List<{}>", scalar_param_to_kotlin(inner, custom_types))
            }
        }
        FfiParamType::Option(inner) => format!("{}?", scalar_param_to_kotlin(inner, custom_types)),
    }
}

pub fn return_to_kotlin(ty: &FfiReturnType, custom_types: &HashMap<String, String>) -> String {
    match ty {
        FfiReturnType::Unit => String::new(),
        FfiReturnType::Primitive(p) => resolve_primitive(p, custom_types),
        FfiReturnType::Complex(name) => name.clone(),
        FfiReturnType::Owned(name) => name.clone(),
        FfiReturnType::Vec(inner) => {
            if matches!(inner, FfiScalarReturn::Primitive(p) if p == "u8") {
                "ByteArray".into()
            } else {
                format!("List<{}>", scalar_return_to_kotlin(inner, custom_types))
            }
        }
        FfiReturnType::Option(inner) => {
            format!("{}?", scalar_return_to_kotlin(inner, custom_types))
        }
    }
}

fn scalar_param_to_kotlin(ty: &FfiScalarParam, custom_types: &HashMap<String, String>) -> String {
    match ty {
        FfiScalarParam::Primitive(p) => resolve_primitive(p, custom_types),
        FfiScalarParam::Complex(name) => name.clone(),
    }
}

fn scalar_return_to_kotlin(ty: &FfiScalarReturn, custom_types: &HashMap<String, String>) -> String {
    match ty {
        FfiScalarReturn::Primitive(p) => resolve_primitive(p, custom_types),
        FfiScalarReturn::Complex(name) => name.clone(),
        FfiScalarReturn::Owned(name) => name.clone(),
    }
}

pub fn resolve_primitive(name: &str, custom_types: &HashMap<String, String>) -> String {
    let resolved = custom_types.get(name).map(|s| s.as_str()).unwrap_or(name);
    match resolved {
        "bool" => "Boolean".into(),
        "u8" | "u16" | "u32" | "i8" | "i16" | "i32" => "Int".into(),
        "u64" | "i64" | "usize" => "Long".into(),
        "f32" => "Float".into(),
        "f64" => "Double".into(),
        "String" => "String".into(),
        other => other.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::*;

    fn empty_ct() -> HashMap<String, String> {
        HashMap::new()
    }

    fn with_platform_handle() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("PlatformHandle".into(), "u64".into());
        m
    }

    #[test]
    fn interface_skips_constructor() {
        let iface = FfiInterface {
            name: "EditorHost".into(),
            methods: vec![
                FfiMethod {
                    name: "create".into(),
                    is_async: true,
                    is_constructor: true,
                    params: vec![],
                    return_type: FfiReturnType::Owned("EditorHost".into()),
                },
                FfiMethod {
                    name: "load_icu_data".into(),
                    is_async: false,
                    is_constructor: false,
                    params: vec![FfiParam {
                        name: "data".into(),
                        ty: FfiParamType::Vec(FfiScalarParam::Primitive("u8".into())),
                    }],
                    return_type: FfiReturnType::Unit,
                },
            ],
        };
        let output = generate_interface(&iface, &empty_ct());
        assert!(!output.contains("fun create("));
        assert!(output.contains("fun loadIcuData(data: ByteArray)"));
    }

    #[test]
    fn interface_complex_types() {
        let iface = FfiInterface {
            name: "Editor".into(),
            methods: vec![
                FfiMethod {
                    name: "enqueue".into(),
                    is_async: false,
                    is_constructor: false,
                    params: vec![FfiParam {
                        name: "message".into(),
                        ty: FfiParamType::Complex("Message".into()),
                    }],
                    return_type: FfiReturnType::Unit,
                },
                FfiMethod {
                    name: "tick".into(),
                    is_async: false,
                    is_constructor: false,
                    params: vec![],
                    return_type: FfiReturnType::Vec(FfiScalarReturn::Complex("EditorEvent".into())),
                },
                FfiMethod {
                    name: "cursor".into(),
                    is_async: false,
                    is_constructor: false,
                    params: vec![],
                    return_type: FfiReturnType::Option(FfiScalarReturn::Complex("PageRect".into())),
                },
            ],
        };
        let output = generate_interface(&iface, &empty_ct());
        assert!(output.contains("fun enqueue(message: Message)"));
        assert!(output.contains("fun tick(): List<EditorEvent>"));
        assert!(output.contains("fun cursor(): PageRect?"));
    }

    #[test]
    fn resolve_platform_handle_via_custom_types() {
        let ct = with_platform_handle();
        assert_eq!(resolve_primitive("PlatformHandle", &ct), "Long");
        assert_eq!(
            param_to_kotlin(&FfiParamType::Primitive("PlatformHandle".into()), &ct),
            "Long"
        );
    }

    #[test]
    fn interface_with_platform_handle() {
        let iface = FfiInterface {
            name: "Editor".into(),
            methods: vec![FfiMethod {
                name: "attach_surface".into(),
                is_async: false,
                is_constructor: false,
                params: vec![
                    FfiParam {
                        name: "page".into(),
                        ty: FfiParamType::Primitive("u32".into()),
                    },
                    FfiParam {
                        name: "handle".into(),
                        ty: FfiParamType::Primitive("PlatformHandle".into()),
                    },
                    FfiParam {
                        name: "width".into(),
                        ty: FfiParamType::Primitive("u32".into()),
                    },
                    FfiParam {
                        name: "height".into(),
                        ty: FfiParamType::Primitive("u32".into()),
                    },
                    FfiParam {
                        name: "scale_factor".into(),
                        ty: FfiParamType::Primitive("f64".into()),
                    },
                ],
                return_type: FfiReturnType::Unit,
            }],
        };
        let ct = with_platform_handle();
        let output = generate_interface(&iface, &ct);
        assert!(output.contains("fun attachSurface(page: Int, handle: Long, width: Int, height: Int, scaleFactor: Double)"));
    }
}
