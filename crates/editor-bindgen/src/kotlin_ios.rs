use std::collections::HashMap;
use std::path::Path;

use heck::ToLowerCamelCase;

use crate::kotlin_iface::{param_to_kotlin, return_to_kotlin};
use crate::meta::{
    FfiInterface, FfiMethod, FfiParam, FfiParamType, FfiReturnType, FfiScalarParam, FfiScalarReturn,
};
use crate::objc::objc_selector;

const PACKAGE: &str = "co.typie.editor.ffi";

pub fn generate_all(
    interfaces: &[FfiInterface],
    custom_types: &HashMap<String, String>,
    output_dir: &Path,
) {
    let pkg_dir = output_dir.join(PACKAGE.replace('.', "/"));
    std::fs::create_dir_all(&pkg_dir).expect("failed to create output directory");

    for iface in interfaces {
        let content = generate_ios_wrapper(iface, interfaces, custom_types);
        let path = pkg_dir.join(format!("Ios{}.kt", iface.name));
        std::fs::write(&path, content).expect("failed to write file");
    }
}

fn generate_ios_wrapper(
    iface: &FfiInterface,
    all_interfaces: &[FfiInterface],
    custom_types: &HashMap<String, String>,
) -> String {
    let mut w = CodeWriter::new();

    w.line("@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class, kotlinx.cinterop.BetaInteropApi::class)");
    w.line("");
    w.line(&format!("package {}", PACKAGE));
    w.line("");
    w.line("import kotlinx.cinterop.ObjCObjectVar");
    w.line("import kotlinx.cinterop.alloc");
    w.line("import kotlinx.cinterop.allocArrayOf");
    w.line("import kotlinx.cinterop.memScoped");
    w.line("import kotlinx.cinterop.ptr");
    w.line("import kotlinx.cinterop.value");
    w.line("import kotlinx.serialization.json.Json");
    w.line("import platform.Foundation.NSData");
    w.line("import platform.Foundation.NSError");
    w.line("import platform.Foundation.create");
    w.line("import co.typie.editor.EditorException");
    w.line(&format!(
        "import swiftPMImport.co.typie.compose.Native{} as Swift{}",
        iface.name, iface.name
    ));
    w.line("");
    w.line("private val json = Json { ignoreUnknownKeys = true }");
    w.line("");

    w.open_block(&format!(
        "class Ios{}(private val native: Swift{}) : {}",
        iface.name, iface.name, iface.name
    ));

    for method in &iface.methods {
        if method.is_constructor {
            continue;
        }
        w.line("");
        generate_method(&mut w, method, all_interfaces, custom_types);
    }

    let constructors: Vec<_> = iface.methods.iter().filter(|m| m.is_constructor).collect();
    if !constructors.is_empty() {
        w.line("");
        w.open_block("companion object");
        for ctor in &constructors {
            let kt_name = ctor.name.to_lower_camel_case();
            let params_sig = ctor
                .params
                .iter()
                .map(|p| {
                    let kt_param = p.name.to_lower_camel_case();
                    let kt_type = param_to_kotlin(&p.ty, custom_types);
                    let default = if matches!(p.ty, FfiParamType::Option(_)) {
                        " = null"
                    } else {
                        ""
                    };
                    format!("{}: {}{}", kt_param, kt_type, default)
                })
                .collect::<Vec<_>>()
                .join(", ");

            w.line(&format!(
                "fun {}({}): Ios{} {{",
                kt_name, params_sig, iface.name
            ));
            w.indent += 1;
            w.line(&format!("val native = Swift{}()", iface.name));
            w.line("memScoped {");
            w.indent += 1;
            w.line("val error = alloc<ObjCObjectVar<NSError?>>()");
            emit_pre_call_conversions(&mut w, &ctor.params);

            let selector = objc_selector(ctor);
            if ctor.params.is_empty() {
                w.line(&format!("native.{}(error = error.ptr)", selector));
            } else {
                let cinterop_args = crate::objc::kotlin_cinterop_args(&ctor.params, |p| {
                    convert_param_value(p, custom_types)
                });
                w.line(&format!(
                    "native.{}({}, error = error.ptr)",
                    selector, cinterop_args
                ));
            }

            w.line("error.value?.let { throw EditorException(it.localizedDescription) }");
            w.indent -= 1;
            w.line("}");
            w.line(&format!("return Ios{}(native)", iface.name));
            w.indent -= 1;
            w.line("}");
        }
        w.close_block();
    }

    w.close_block();

    w.finish()
}

fn generate_method(
    w: &mut CodeWriter,
    method: &FfiMethod,
    all_interfaces: &[FfiInterface],
    custom_types: &HashMap<String, String>,
) {
    let kt_name = method.name.to_lower_camel_case();
    let params_sig = method
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
    let is_unit = ret.is_empty();
    if is_unit {
        w.line(&format!("override fun {}({}) {{", kt_name, params_sig));
    } else {
        w.line(&format!(
            "override fun {}({}): {} {{",
            kt_name, params_sig, ret
        ));
    }
    w.indent += 1;
    if is_unit {
        w.line("memScoped {");
    } else {
        w.line("return memScoped {");
    }
    w.indent += 1;

    w.line("val error = alloc<ObjCObjectVar<NSError?>>()");

    let selector = objc_selector(method);
    let call_args = build_call_args(method, custom_types);

    let native_call = if call_args.is_empty() {
        format!("native.{}(error = error.ptr)", selector)
    } else {
        format!(
            "native.{}(\n{}        error = error.ptr,\n    )",
            selector,
            format_call_args_multiline(&call_args)
        )
    };

    match &method.return_type {
        FfiReturnType::Unit => {
            emit_pre_call_conversions(w, &method.params);
            w.line(&format!("{}", native_call));
            w.line("error.value?.let { throw EditorException(it.localizedDescription) }");
        }
        FfiReturnType::Complex(_) => {
            emit_pre_call_conversions(w, &method.params);
            w.line(&format!("val result = {}", native_call));
            w.line("error.value?.let { throw EditorException(it.localizedDescription) }");
            w.line("json.decodeFromString(result!!)");
        }
        FfiReturnType::Vec(inner) => match inner {
            FfiScalarReturn::Primitive(_) => {
                emit_pre_call_conversions(w, &method.params);
                w.line(&format!("val result = {}", native_call));
                w.line("error.value?.let { throw EditorException(it.localizedDescription) }");
                w.line("result!!");
            }
            FfiScalarReturn::Complex(_) => {
                emit_pre_call_conversions(w, &method.params);
                w.line(&format!("val result = {}", native_call));
                w.line("error.value?.let { throw EditorException(it.localizedDescription) }");
                w.line("@Suppress(\"UNCHECKED_CAST\") (result!! as List<String>).map { json.decodeFromString(it) }");
            }
            FfiScalarReturn::Owned(name) => {
                emit_pre_call_conversions(w, &method.params);
                let ios_class = format!("Ios{}", name);
                w.line(&format!("val result = {}", native_call));
                w.line("error.value?.let { throw EditorException(it.localizedDescription) }");
                w.line(&format!("result!!.map {{ {}(it) }}", ios_class));
            }
        },
        FfiReturnType::Option(inner) => match inner {
            FfiScalarReturn::Primitive(_) => {
                emit_pre_call_conversions(w, &method.params);
                w.line(&format!("val result = {}", native_call));
                w.line("error.value?.let { throw EditorException(it.localizedDescription) }");
                w.line("result");
            }
            FfiScalarReturn::Complex(_) => {
                emit_pre_call_conversions(w, &method.params);
                w.line(&format!("val result = {}", native_call));
                w.line("error.value?.let { throw EditorException(it.localizedDescription) }");
                w.line("result?.let { json.decodeFromString(it) }");
            }
            FfiScalarReturn::Owned(name) => {
                emit_pre_call_conversions(w, &method.params);
                let ios_class = format!("Ios{}", name);
                w.line(&format!("val result = {}", native_call));
                w.line("error.value?.let { throw EditorException(it.localizedDescription) }");
                w.line(&format!("result?.let {{ {}(it) }}", ios_class));
            }
        },
        FfiReturnType::Owned(name) => {
            let is_iface = all_interfaces.iter().any(|i| &i.name == name);
            emit_pre_call_conversions(w, &method.params);
            w.line(&format!("val result = {}", native_call));
            w.line("error.value?.let { throw EditorException(it.localizedDescription) }");
            if is_iface {
                w.line(&format!("Ios{}(result!!)", name));
            } else {
                w.line("result!!");
            }
        }
        FfiReturnType::Primitive(_) => {
            emit_pre_call_conversions(w, &method.params);
            w.line(&format!("val result = {}", native_call));
            w.line("error.value?.let { throw EditorException(it.localizedDescription) }");
            w.line("result!!");
        }
    }

    w.indent -= 1;
    w.line("}");
    w.indent -= 1;
    w.line("}");
}

fn emit_pre_call_conversions(w: &mut CodeWriter, params: &[FfiParam]) {
    for param in params {
        if is_byte_array(&param.ty) {
            let kt_name = param.name.to_lower_camel_case();
            w.line(&format!(
                "val {}NsData = NSData.create(bytes = allocArrayOf({}), length = {}.size.toULong())",
                kt_name, kt_name, kt_name
            ));
        }
    }
}

fn build_call_args(
    method: &FfiMethod,
    custom_types: &HashMap<String, String>,
) -> Vec<(usize, String, String)> {
    method
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let label = p.name.to_lower_camel_case();
            let value = convert_param_value(p, custom_types);
            (i, label, value)
        })
        .collect()
}

fn convert_param_value(param: &FfiParam, _custom_types: &HashMap<String, String>) -> String {
    let kt_name = param.name.to_lower_camel_case();
    match &param.ty {
        FfiParamType::Complex(_) => format!("json.encodeToString({})", kt_name),
        FfiParamType::Vec(inner) => {
            if matches!(inner, FfiScalarParam::Primitive(p) if p == "u8") {
                format!("{}NsData", kt_name)
            } else if matches!(inner, FfiScalarParam::Complex(_)) {
                format!("{}.map {{ json.encodeToString(it) }}", kt_name)
            } else {
                kt_name
            }
        }
        FfiParamType::Option(inner) => {
            if matches!(inner, FfiScalarParam::Complex(_)) {
                format!("{}?.let {{ json.encodeToString(it) }}", kt_name)
            } else {
                kt_name
            }
        }
        FfiParamType::Primitive(_) => kt_name,
    }
}

fn is_byte_array(ty: &FfiParamType) -> bool {
    matches!(ty, FfiParamType::Vec(FfiScalarParam::Primitive(p)) if p == "u8")
}

fn format_call_args_multiline(args: &[(usize, String, String)]) -> String {
    let mut lines = String::new();
    for (i, label, value) in args {
        if *i == 0 {
            lines.push_str(&format!("    {},\n    ", value));
        } else {
            lines.push_str(&format!("{} = {},\n    ", label, value));
        }
    }
    lines
}

struct CodeWriter {
    buf: String,
    indent: usize,
}

impl CodeWriter {
    fn new() -> Self {
        Self {
            buf: String::new(),
            indent: 0,
        }
    }

    fn line(&mut self, s: &str) {
        if s.is_empty() {
            self.buf.push('\n');
        } else {
            for _ in 0..self.indent {
                self.buf.push_str("    ");
            }
            self.buf.push_str(s);
            self.buf.push('\n');
        }
    }

    fn open_block(&mut self, header: &str) {
        self.line(&format!("{} {{", header));
        self.indent += 1;
    }

    fn close_block(&mut self) {
        self.indent -= 1;
        self.line("}");
    }

    fn finish(self) -> String {
        self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::{FfiParam, FfiParamType, FfiReturnType, FfiScalarParam, FfiScalarReturn};

    fn empty_ct() -> HashMap<String, String> {
        HashMap::new()
    }

    fn make_method(name: &str, params: Vec<FfiParam>, return_type: FfiReturnType) -> FfiMethod {
        FfiMethod {
            name: name.into(),
            is_async: false,
            is_constructor: false,
            params,
            return_type,
        }
    }

    fn make_param(name: &str, ty: FfiParamType) -> FfiParam {
        FfiParam {
            name: name.into(),
            ty,
        }
    }

    #[test]
    fn selector_load_font_base_with_family() {
        let method = make_method(
            "load_font_base",
            vec![
                make_param("family", FfiParamType::Primitive("String".into())),
                make_param("weight", FfiParamType::Primitive("u16".into())),
                make_param(
                    "data",
                    FfiParamType::Vec(FfiScalarParam::Primitive("u8".into())),
                ),
            ],
            FfiReturnType::Unit,
        );
        assert_eq!(objc_selector(&method), "loadFontBaseWithFamily");
    }

    #[test]
    fn selector_tick_with_error() {
        let method = make_method(
            "tick",
            vec![],
            FfiReturnType::Vec(FfiScalarReturn::Complex("EditorEvent".into())),
        );
        assert_eq!(objc_selector(&method), "tickWithError");
    }

    #[test]
    fn byte_array_param_generates_nsdata() {
        let iface = FfiInterface {
            name: "EditorHost".into(),
            methods: vec![make_method(
                "load_icu_data",
                vec![make_param(
                    "data",
                    FfiParamType::Vec(FfiScalarParam::Primitive("u8".into())),
                )],
                FfiReturnType::Unit,
            )],
        };
        let output = generate_ios_wrapper(&iface, &[iface.clone()], &empty_ct());
        assert!(
            output.contains(
                "NSData.create(bytes = allocArrayOf(data), length = data.size.toULong())"
            )
        );
        assert!(output.contains("dataNsData"));
    }

    #[test]
    fn every_method_has_mem_scoped_and_error_handling() {
        let iface = FfiInterface {
            name: "Editor".into(),
            methods: vec![
                make_method(
                    "tick",
                    vec![],
                    FfiReturnType::Vec(FfiScalarReturn::Complex("EditorEvent".into())),
                ),
                make_method(
                    "enqueue",
                    vec![make_param(
                        "message",
                        FfiParamType::Complex("Message".into()),
                    )],
                    FfiReturnType::Unit,
                ),
            ],
        };
        let output = generate_ios_wrapper(&iface, &[iface.clone()], &empty_ct());
        assert!(output.contains("memScoped {"));
        assert!(output.contains("alloc<ObjCObjectVar<NSError?>>()"));
        assert!(
            output.contains("error.value?.let { throw EditorException(it.localizedDescription) }")
        );
    }

    #[test]
    fn owned_return_type_generates_ios_wrapper() {
        let editor_iface = FfiInterface {
            name: "Editor".into(),
            methods: vec![],
        };
        let host_iface = FfiInterface {
            name: "EditorHost".into(),
            methods: vec![make_method(
                "create_editor",
                vec![
                    make_param("doc", FfiParamType::Complex("Doc".into())),
                    make_param("selection", FfiParamType::Complex("Selection".into())),
                    make_param("viewport", FfiParamType::Complex("Viewport".into())),
                ],
                FfiReturnType::Owned("Editor".into()),
            )],
        };
        let all_interfaces = vec![editor_iface.clone(), host_iface.clone()];
        let output = generate_ios_wrapper(&host_iface, &all_interfaces, &empty_ct());
        assert!(output.contains("IosEditor(result!!)"));
    }

    #[test]
    fn generated_file_has_correct_imports_and_class() {
        let iface = FfiInterface {
            name: "EditorHost".into(),
            methods: vec![],
        };
        let output = generate_ios_wrapper(&iface, &[iface.clone()], &empty_ct());
        assert!(output.contains("package co.typie.editor.ffi"));
        assert!(output.contains("import kotlinx.cinterop.memScoped"));
        assert!(output.contains("import platform.Foundation.NSError"));
        assert!(
            output.contains(
                "import swiftPMImport.co.typie.compose.NativeEditorHost as SwiftEditorHost"
            )
        );
        assert!(output.contains("private val json = Json { ignoreUnknownKeys = true }"));
        assert!(
            output.contains(
                "class IosEditorHost(private val native: SwiftEditorHost) : EditorHost {"
            )
        );
    }

    #[test]
    fn constructor_not_generated_as_override() {
        let iface = FfiInterface {
            name: "EditorHost".into(),
            methods: vec![
                FfiMethod {
                    name: "create".into(),
                    is_async: false,
                    is_constructor: true,
                    params: vec![FfiParam {
                        name: "kind".into(),
                        ty: FfiParamType::Option(FfiScalarParam::Complex("BackendKind".into())),
                    }],
                    return_type: FfiReturnType::Owned("EditorHost".into()),
                },
                make_method("tick", vec![], FfiReturnType::Unit),
            ],
        };
        let output = generate_ios_wrapper(&iface, &[iface.clone()], &empty_ct());
        assert!(
            !output.contains("override fun create("),
            "Constructor should not be an override method:\n{}",
            output
        );
        assert!(output.contains("fun tick("));
    }

    #[test]
    fn constructor_generates_companion_factory() {
        let iface = FfiInterface {
            name: "EditorHost".into(),
            methods: vec![FfiMethod {
                name: "create".into(),
                is_async: false,
                is_constructor: true,
                params: vec![FfiParam {
                    name: "kind".into(),
                    ty: FfiParamType::Option(FfiScalarParam::Complex("BackendKind".into())),
                }],
                return_type: FfiReturnType::Owned("EditorHost".into()),
            }],
        };
        let output = generate_ios_wrapper(&iface, &[iface.clone()], &empty_ct());
        assert!(
            output.contains("companion object"),
            "Expected companion object:\n{}",
            output
        );
        assert!(
            output.contains("fun create("),
            "Expected fun create:\n{}",
            output
        );
        assert!(
            !output.contains("suspend fun create("),
            "Constructor should not be suspend:\n{}",
            output
        );
        assert!(
            output.contains("kind: BackendKind? = null"),
            "Expected typed BackendKind? param with default null:\n{}",
            output
        );
        assert!(
            output.contains("SwiftEditorHost()"),
            "Expected Swift native object creation:\n{}",
            output
        );
        assert!(
            output.contains("createWithKind("),
            "Expected ObjC cinterop selector:\n{}",
            output
        );
        assert!(
            output.contains("error = error.ptr"),
            "Expected error pointer argument:\n{}",
            output
        );
        assert!(
            output.contains("json.encodeToString(it)"),
            "Expected JSON serialization:\n{}",
            output
        );
        assert!(
            output.contains("return IosEditorHost(native)"),
            "Expected IosEditorHost wrapper return:\n{}",
            output
        );
    }

    #[test]
    fn constructor_with_byte_array_emits_mem_scope_and_nsdata() {
        let iface = FfiInterface {
            name: "EditorHost".into(),
            methods: vec![FfiMethod {
                name: "create".into(),
                is_async: false,
                is_constructor: true,
                params: vec![
                    FfiParam {
                        name: "kind".into(),
                        ty: FfiParamType::Option(FfiScalarParam::Complex("BackendKind".into())),
                    },
                    FfiParam {
                        name: "icu_data".into(),
                        ty: FfiParamType::Vec(FfiScalarParam::Primitive("u8".into())),
                    },
                ],
                return_type: FfiReturnType::Owned("EditorHost".into()),
            }],
        };
        let output = generate_ios_wrapper(&iface, &[iface.clone()], &empty_ct());
        assert!(
            output.contains("memScoped {"),
            "Expected memScoped block for ByteArray conversion:\n{}",
            output
        );
        assert!(
            output.contains("val icuDataNsData = NSData.create(bytes = allocArrayOf(icuData), length = icuData.size.toULong())"),
            "Expected NSData conversion for ByteArray param:\n{}",
            output
        );
        assert!(
            output.contains("icuData = icuDataNsData"),
            "Expected icuDataNsData passed to native call:\n{}",
            output
        );
    }

    #[test]
    fn vec_complex_return_generates_unchecked_cast_and_map() {
        let iface = FfiInterface {
            name: "Editor".into(),
            methods: vec![make_method(
                "tick",
                vec![],
                FfiReturnType::Vec(FfiScalarReturn::Complex("EditorEvent".into())),
            )],
        };
        let output = generate_ios_wrapper(&iface, &[iface.clone()], &empty_ct());
        assert!(output.contains("@Suppress(\"UNCHECKED_CAST\")"));
        assert!(output.contains("result!! as List<String>"));
        assert!(output.contains(".map { json.decodeFromString(it) }"));
    }

    #[test]
    fn option_complex_return_generates_if_empty_let_decode() {
        let iface = FfiInterface {
            name: "Editor".into(),
            methods: vec![make_method(
                "cursor",
                vec![],
                FfiReturnType::Option(FfiScalarReturn::Complex("CursorRect".into())),
            )],
        };
        let output = generate_ios_wrapper(&iface, &[iface.clone()], &empty_ct());
        assert!(output.contains("result?.let { json.decodeFromString(it) }"));
    }
}
