use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let base = match args.as_slice() {
        [base] => base.as_str(),
        _ => {
            eprintln!("Usage: editor-bindgen-js <name>");
            process::exit(1);
        }
    };

    let js_src = read_file(&format!("{base}.js"));
    let dts_src = read_file(&format!("{base}.d.ts"));

    let parsed = parse_js(&js_src);

    write_file(&format!("{base}.js"), &generate_js(&parsed));
    write_file(
        &format!("{base}.d.ts"),
        &generate_dts(&dts_src, &parsed.export_names),
    );

    eprintln!(
        "Transformed {len} exports: {names}",
        len = parsed.export_names.len(),
        names = parsed.export_names.join(", "),
    );
}

struct ParsedJs {
    body_lines: Vec<String>,
    export_names: Vec<String>,
    has_start: bool,
}

fn parse_js(src: &str) -> ParsedJs {
    let lines: Vec<&str> = src.lines().collect();

    let import_idx = lines
        .iter()
        .position(|l| l.starts_with("import source "))
        .unwrap_or_else(|| fail("`import source` not found. Is this `--target module` output?"));

    let init_block = &lines[import_idx..];
    if !init_block
        .iter()
        .any(|l| l.contains("new WebAssembly.Instance("))
    {
        fail("`new WebAssembly.Instance(...)` not found in init block.");
    }

    let has_start = init_block.iter().any(|l| l.contains("__wbindgen_start()"));

    let mut export_names: Vec<String> = Vec::new();
    let mut body_lines: Vec<String> = Vec::new();

    for line in &lines[..import_idx] {
        if line.contains("@ts-self-types") {
            continue;
        }

        if let Some(name) = parse_export_name(line) {
            export_names.push(name);
            body_lines.push(line.strip_prefix("export ").unwrap().to_string());
        } else {
            body_lines.push(line.to_string());
        }
    }

    if export_names.is_empty() {
        fail("No exports found.");
    }

    ParsedJs {
        body_lines,
        export_names,
        has_start,
    }
}

fn generate_js(parsed: &ParsedJs) -> String {
    let body = parsed.body_lines.join("\n");
    let exports = parsed.export_names.join(", ");
    let start = if parsed.has_start {
        "wasm.__wbindgen_start();\n"
    } else {
        ""
    };

    format!(
        "\
export async function createInstance(wasmModule) {{
let wasm;

{body}

const __instance = await WebAssembly.instantiate(wasmModule, __wbg_get_imports());
wasm = __instance.exports;
{start}
return {{ {exports} }};
}}
"
    )
}

fn generate_dts(src: &str, export_names: &[String]) -> String {
    let declarations = src
        .lines()
        .map(|l| match l.strip_prefix("export class ") {
            Some(rest) => format!("declare class {rest}"),
            None => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");

    let exports = export_names.join(", ");
    let members = export_names
        .iter()
        .map(|n| format!("    {n}: typeof {n};"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "\
{declarations}

export type {{ {exports} }};

export function createInstance(wasmModule: WebAssembly.Module): Promise<{{
{members}
}}>;
"
    )
}

fn parse_export_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("export ")?;
    let rest = rest
        .strip_prefix("class ")
        .or_else(|| rest.strip_prefix("function "))
        .or_else(|| rest.strip_prefix("const "))
        .or_else(|| rest.strip_prefix("let "))?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() { None } else { Some(name) }
}

fn read_file(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| fail(&format!("cannot read {path}: {e}")))
}

fn write_file(path: &str, content: &str) {
    fs::write(path, content).unwrap_or_else(|e| fail(&format!("cannot write {path}: {e}")))
}

fn fail(msg: &str) -> ! {
    eprintln!("ERROR: {msg}");
    process::exit(1)
}
