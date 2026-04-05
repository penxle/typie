use std::collections::HashMap;
use std::path::Path;

use heck::ToLowerCamelCase;

use crate::meta::{FfiInterface, FfiParamType, FfiReturnType, FfiScalarParam, FfiScalarReturn};

const PACKAGE: &str = "co.typie.editor.ffi";

pub fn generate_all(
    interfaces: &[FfiInterface],
    custom_types: &HashMap<String, String>,
    output_dir: &Path,
) {
    let pkg_dir = output_dir.join(PACKAGE.replace('.', "/"));
    std::fs::create_dir_all(&pkg_dir).expect("failed to create output directory");

    for iface in interfaces {
        let content = generate_jna_class(iface, interfaces, custom_types);
        let path = pkg_dir.join(format!("Jna{}.kt", iface.name));
        std::fs::write(&path, content).expect("failed to write file");
    }
}

fn generate_jna_class(
    iface: &FfiInterface,
    all_interfaces: &[FfiInterface],
    custom_types: &HashMap<String, String>,
) -> String {
    let mut out = String::new();

    out.push_str(&format!("package {}\n\n", PACKAGE));
    out.push_str("import kotlinx.serialization.json.Json\n");
    out.push_str(&format!(
        "import uniffi.editor_ffi.{} as Native{}\n",
        iface.name, iface.name
    ));
    out.push_str("import uniffi.editor_ffi.EditorException as NativeEditorException\n");
    out.push_str("import co.typie.editor.EditorException\n");
    out.push('\n');
    out.push_str("private val json = Json { ignoreUnknownKeys = true }\n");
    out.push('\n');

    out.push_str(&format!(
        "class Jna{}(private val native: Native{}) : {} {{\n",
        iface.name, iface.name, iface.name
    ));

    for method in &iface.methods {
        if method.is_constructor {
            continue;
        }

        let kt_name = method.name.to_lower_camel_case();
        let params = method
            .params
            .iter()
            .map(|p| {
                format!(
                    "{}: {}",
                    p.name.to_lower_camel_case(),
                    crate::kotlin_iface::param_to_kotlin(&p.ty, custom_types)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        let ret_kt = crate::kotlin_iface::return_to_kotlin(&method.return_type, custom_types);
        let sig = if ret_kt.is_empty() {
            format!("    override fun {}({})", kt_name, params)
        } else {
            format!("    override fun {}({}): {}", kt_name, params, ret_kt)
        };

        out.push_str(&format!("{} {{\n", sig));
        out.push_str("        try {\n");

        let native_args = method
            .params
            .iter()
            .map(|p| {
                let kt_param = p.name.to_lower_camel_case();
                convert_param(&p.ty, &kt_param, custom_types)
            })
            .collect::<Vec<_>>()
            .join(",\n                ");

        let native_call = if method.params.is_empty() {
            format!("native.{}()", kt_name)
        } else {
            format!(
                "native.{}(\n                {}\n            )",
                kt_name, native_args
            )
        };

        let return_stmt = build_return_stmt(&method.return_type, &native_call, all_interfaces);
        out.push_str(&format!("            {}\n", return_stmt));

        out.push_str("        } catch (e: NativeEditorException) {\n");
        out.push_str("            throw EditorException(e.message ?: \"Unknown editor error\")\n");
        out.push_str("        }\n");
        out.push_str("    }\n");
    }

    let constructors: Vec<_> = iface.methods.iter().filter(|m| m.is_constructor).collect();
    if !constructors.is_empty() {
        out.push_str("    companion object {\n");
        for ctor in &constructors {
            let kt_name = ctor.name.to_lower_camel_case();
            let params = ctor
                .params
                .iter()
                .map(|p| {
                    let kt_param = p.name.to_lower_camel_case();
                    let kt_type = crate::kotlin_iface::param_to_kotlin(&p.ty, custom_types);
                    let default = if matches!(p.ty, FfiParamType::Option(_)) {
                        " = null"
                    } else {
                        ""
                    };
                    format!("{}: {}{}", kt_param, kt_type, default)
                })
                .collect::<Vec<_>>()
                .join(", ");

            let suspend = if ctor.is_async { "suspend " } else { "" };
            out.push_str(&format!(
                "        {}fun {}({}): Jna{} {{\n",
                suspend, kt_name, params, iface.name
            ));
            out.push_str("            try {\n");

            let native_args = ctor
                .params
                .iter()
                .map(|p| {
                    let kt_param = p.name.to_lower_camel_case();
                    convert_param(&p.ty, &kt_param, custom_types)
                })
                .collect::<Vec<_>>()
                .join(",\n                    ");

            let native_call = if ctor.params.is_empty() {
                format!("Native{}.{}()", iface.name, kt_name)
            } else {
                format!(
                    "Native{}.{}(\n                    {}\n                )",
                    iface.name, kt_name, native_args
                )
            };

            out.push_str(&format!(
                "                return Jna{}({})\n",
                iface.name, native_call
            ));
            out.push_str("            } catch (e: NativeEditorException) {\n");
            out.push_str(
                "                throw EditorException(e.message ?: \"Unknown editor error\")\n",
            );
            out.push_str("            }\n");
            out.push_str("        }\n");
        }
        out.push_str("    }\n");
    }

    out.push_str("}\n");
    out
}

fn convert_param(
    ty: &FfiParamType,
    kt_name: &str,
    custom_types: &HashMap<String, String>,
) -> String {
    match ty {
        FfiParamType::Primitive(name) => jna_primitive_conversion(name, kt_name, custom_types),
        FfiParamType::Complex(_) => format!("json.encodeToString({})", kt_name),
        FfiParamType::Vec(inner) => match inner {
            FfiScalarParam::Primitive(p) if p == "u8" => kt_name.into(),
            FfiScalarParam::Primitive(p) => {
                let conv = jna_primitive_conversion(p, "it", custom_types);
                if conv == "it" {
                    kt_name.into()
                } else {
                    format!("{}.map {{ {} }}", kt_name, conv)
                }
            }
            FfiScalarParam::Complex(_) => {
                format!("{}.map {{ json.encodeToString(it) }}", kt_name)
            }
        },
        FfiParamType::Option(inner) => match inner {
            FfiScalarParam::Primitive(p) => {
                let conv = jna_primitive_conversion(p, "it", custom_types);
                if conv == "it" {
                    kt_name.into()
                } else {
                    format!("{}?.let {{ {} }}", kt_name, conv)
                }
            }
            FfiScalarParam::Complex(_) => {
                format!("{}?.let {{ json.encodeToString(it) }}", kt_name)
            }
        },
    }
}

fn jna_primitive_conversion(
    name: &str,
    kt_name: &str,
    custom_types: &HashMap<String, String>,
) -> String {
    let resolved = custom_types.get(name).map(|s| s.as_str()).unwrap_or(name);
    match resolved {
        "u32" => format!("{}.toUInt()", kt_name),
        "u16" => format!("{}.toUShort()", kt_name),
        "u64" => format!("{}.toULong()", kt_name),
        _ => kt_name.into(),
    }
}

fn build_return_stmt(
    return_type: &FfiReturnType,
    native_call: &str,
    all_interfaces: &[FfiInterface],
) -> String {
    match return_type {
        FfiReturnType::Unit => native_call.into(),
        FfiReturnType::Primitive(_) => format!("return {}", native_call),
        FfiReturnType::Complex(_) => {
            format!("return json.decodeFromString({})", native_call)
        }
        FfiReturnType::Owned(name) => {
            if all_interfaces.iter().any(|i| &i.name == name) {
                format!("return Jna{}({})", name, native_call)
            } else {
                format!("return {}", native_call)
            }
        }
        FfiReturnType::Vec(inner) => match inner {
            FfiScalarReturn::Primitive(p) if p == "u8" => {
                format!("return {}", native_call)
            }
            FfiScalarReturn::Primitive(_) => format!("return {}", native_call),
            FfiScalarReturn::Complex(_) => {
                format!("return {}.map {{ json.decodeFromString(it) }}", native_call)
            }
            FfiScalarReturn::Owned(name) => {
                format!("return {}.map {{ Jna{}(it) }}", native_call, name)
            }
        },
        FfiReturnType::Option(inner) => match inner {
            FfiScalarReturn::Primitive(_) => format!("return {}", native_call),
            FfiScalarReturn::Complex(_) => {
                format!(
                    "return {}?.let {{ json.decodeFromString(it) }}",
                    native_call
                )
            }
            FfiScalarReturn::Owned(name) => {
                format!("return {}?.let {{ Jna{}(it) }}", native_call, name)
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::{
        FfiInterface, FfiMethod, FfiParam, FfiParamType, FfiReturnType, FfiScalarParam,
        FfiScalarReturn,
    };

    fn empty_ct() -> HashMap<String, String> {
        HashMap::new()
    }

    fn with_platform_handle() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("PlatformHandle".into(), "u64".into());
        m
    }

    fn editor_host_iface() -> FfiInterface {
        FfiInterface {
            name: "EditorHost".into(),
            methods: vec![
                FfiMethod {
                    name: "create".into(),
                    is_async: true,
                    is_constructor: true,
                    params: vec![FfiParam {
                        name: "kind".into(),
                        ty: FfiParamType::Option(FfiScalarParam::Complex("BackendKind".into())),
                    }],
                    return_type: FfiReturnType::Owned("EditorHost".into()),
                },
                FfiMethod {
                    name: "create_editor".into(),
                    is_async: false,
                    is_constructor: false,
                    params: vec![
                        FfiParam {
                            name: "doc".into(),
                            ty: FfiParamType::Complex("Doc".into()),
                        },
                        FfiParam {
                            name: "selection".into(),
                            ty: FfiParamType::Complex("Selection".into()),
                        },
                        FfiParam {
                            name: "viewport".into(),
                            ty: FfiParamType::Complex("Viewport".into()),
                        },
                    ],
                    return_type: FfiReturnType::Owned("Editor".into()),
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
                FfiMethod {
                    name: "load_font_base".into(),
                    is_async: false,
                    is_constructor: false,
                    params: vec![
                        FfiParam {
                            name: "family".into(),
                            ty: FfiParamType::Primitive("String".into()),
                        },
                        FfiParam {
                            name: "weight".into(),
                            ty: FfiParamType::Primitive("u16".into()),
                        },
                        FfiParam {
                            name: "data".into(),
                            ty: FfiParamType::Vec(FfiScalarParam::Primitive("u8".into())),
                        },
                    ],
                    return_type: FfiReturnType::Unit,
                },
            ],
        }
    }

    fn editor_iface() -> FfiInterface {
        FfiInterface {
            name: "Editor".into(),
            methods: vec![FfiMethod {
                name: "tick".into(),
                is_async: false,
                is_constructor: false,
                params: vec![],
                return_type: FfiReturnType::Vec(FfiScalarReturn::Complex("EditorEvent".into())),
            }],
        }
    }

    #[test]
    fn load_font_base_u16_param_converts_to_ushort() {
        let iface = editor_host_iface();
        let all_ifaces = vec![iface.clone(), editor_iface()];
        let output = generate_jna_class(&iface, &all_ifaces, &empty_ct());
        assert!(
            output.contains("weight.toUShort()"),
            "Expected weight.toUShort() in output:\n{}",
            output
        );
    }

    #[test]
    fn create_editor_owned_return_wraps_in_jna_editor() {
        let iface = editor_host_iface();
        let all_ifaces = vec![iface.clone(), editor_iface()];
        let output = generate_jna_class(&iface, &all_ifaces, &empty_ct());
        assert!(
            output.contains("return JnaEditor("),
            "Expected return JnaEditor(...) in output:\n{}",
            output
        );
    }

    #[test]
    fn tick_vec_complex_return_maps_with_json_decode() {
        let iface = editor_iface();
        let all_ifaces = vec![editor_host_iface(), iface.clone()];
        let output = generate_jna_class(&iface, &all_ifaces, &empty_ct());
        assert!(
            output.contains(".map { json.decodeFromString(it) }"),
            "Expected .map {{ json.decodeFromString(it) }} in output:\n{}",
            output
        );
    }

    #[test]
    fn platform_handle_resolves_to_ulong() {
        let ct = with_platform_handle();
        let result = jna_primitive_conversion("PlatformHandle", "handle", &ct);
        assert_eq!(result, "handle.toULong()");
    }

    #[test]
    fn constructor_not_generated_as_override() {
        let iface = editor_host_iface();
        let all_ifaces = vec![iface.clone()];
        let output = generate_jna_class(&iface, &all_ifaces, &empty_ct());
        assert!(
            !output.contains("override fun create("),
            "Constructor should not be an override method:\n{}",
            output
        );
    }

    #[test]
    fn constructor_generates_companion_factory() {
        let iface = editor_host_iface();
        let all_ifaces = vec![iface.clone()];
        let output = generate_jna_class(&iface, &all_ifaces, &empty_ct());
        assert!(
            output.contains("companion object"),
            "Expected companion object:\n{}",
            output
        );
        assert!(
            output.contains("suspend fun create("),
            "Expected suspend fun create:\n{}",
            output
        );
        assert!(
            output.contains("kind: BackendKind? = null"),
            "Expected typed BackendKind? param with default null:\n{}",
            output
        );
        assert!(
            output.contains("NativeEditorHost.create("),
            "Expected delegation to NativeEditorHost.create:\n{}",
            output
        );
        assert!(
            output.contains("return JnaEditorHost("),
            "Expected return JnaEditorHost wrapper:\n{}",
            output
        );
        assert!(
            output.contains("json.encodeToString(it)"),
            "Expected JSON serialization for Complex param:\n{}",
            output
        );
    }

    #[test]
    fn load_icu_data_bytearray_passed_directly() {
        let iface = editor_host_iface();
        let all_ifaces = vec![iface.clone()];
        let output = generate_jna_class(&iface, &all_ifaces, &empty_ct());
        assert!(
            output.contains("native.loadIcuData("),
            "Expected native.loadIcuData call:\n{}",
            output
        );
        assert!(
            !output.contains("data.toU"),
            "ByteArray data should not be converted:\n{}",
            output
        );
    }
}
