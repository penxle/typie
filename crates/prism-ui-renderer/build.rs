use std::{env, fs, path::PathBuf};

use naga::{
    ShaderStage,
    back::wgsl,
    front::glsl,
    valid::{Capabilities, ValidationFlags, Validator},
};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let source_path = manifest_dir.join("src/shaders/prism-object.frag.glsl");
    println!("cargo:rerun-if-changed={}", source_path.display());
    let source = fs::read_to_string(&source_path).expect("read Prism GLSL source");
    let source = webgl_to_vulkan_glsl(&source);

    let mut frontend = glsl::Frontend::default();
    let module = frontend
        .parse(&glsl::Options::from(ShaderStage::Fragment), &source)
        .unwrap_or_else(|errors| panic!("parse Prism GLSL:\n{errors:#?}\n\n{source}"));
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("validate translated Prism shader");
    let wgsl =
        wgsl::write_string(&module, &info, wgsl::WriterFlags::empty()).expect("write Prism WGSL");
    let wgsl = convert_fragment_origin(&wgsl);
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("prism-object.wgsl");
    fs::write(output, wgsl).expect("write generated Prism WGSL");
}

fn convert_fragment_origin(wgsl: &str) -> String {
    const GENERATED_ASSIGNMENT: &str = "    gl_FragCoord_1 = gl_FragCoord;";
    const WEBGL_COORDINATE_ASSIGNMENT: &str = "    gl_FragCoord_1 = vec4<f32>(gl_FragCoord.x, global.uResolution.y - gl_FragCoord.y, gl_FragCoord.z, gl_FragCoord.w);";

    assert!(
        wgsl.contains(GENERATED_ASSIGNMENT),
        "Naga's generated fragment-coordinate assignment changed"
    );
    wgsl.replacen(GENERATED_ASSIGNMENT, WEBGL_COORDINATE_ASSIGNMENT, 1)
}

fn webgl_to_vulkan_glsl(source: &str) -> String {
    let mut output = String::with_capacity(source.len() + 256);
    let mut uniforms = Vec::new();
    let mut inserted_uniform_block = false;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#version") {
            output.push_str("#version 450 core\n");
        } else if trimmed.starts_with("precision ") {
            continue;
        } else if let Some(declaration) = trimmed.strip_prefix("uniform ") {
            uniforms.push(declaration.to_owned());
        } else {
            if !inserted_uniform_block && trimmed.starts_with("layout(location = 0) out") {
                output.push_str("layout(set = 0, binding = 0, std140) uniform FrameUniforms {\n");
                for declaration in &uniforms {
                    output.push_str("  ");
                    output.push_str(declaration);
                    output.push('\n');
                }
                output.push_str("};\n\n");
                inserted_uniform_block = true;
            }
            if trimmed == "layout(std140) uniform OpticalPaths {" {
                output.push_str("layout(set = 0, binding = 1, std140) uniform OpticalPaths {\n");
            } else {
                output.push_str(line);
                output.push('\n');
            }
        }
    }
    output
}
