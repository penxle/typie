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

    let instance_idx = lines
        .iter()
        .position(|l| l.contains("new WebAssembly.Instance("))
        .unwrap_or_else(|| fail("`new WebAssembly.Instance(...)` not found."));

    if instance_idx <= import_idx {
        fail("Unexpected layout: `new WebAssembly.Instance(...)` must come after `import source`.");
    }

    let has_start = lines[instance_idx..]
        .iter()
        .any(|l| l.contains("__wbindgen_start()"));

    let mut export_names: Vec<String> = Vec::new();
    let mut body_lines: Vec<String> = Vec::new();

    for line in &lines[import_idx + 1..instance_idx] {
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
export async function createInstance(wasmModule, onRuntimeError) {{
let wasm;
let runtimeError;

{body}

const __instance = await WebAssembly.instantiate(wasmModule, __wbg_get_imports());
// Guard the exports themselves, including calls made by generated finalizers.
wasm = Object.fromEntries(Object.entries(__instance.exports).map(([name, value]) => [
    name,
    typeof value !== 'function' ? value : (...args) => {{
        if (runtimeError !== undefined) {{
            if (name.startsWith('__wbg_') && name.endsWith('_free')) return;
            throw runtimeError;
        }}
        try {{
            return value(...args);
        }} catch (error) {{
            if (error instanceof WebAssembly.RuntimeError && runtimeError === undefined) {{
                runtimeError = error;
                try {{
                    onRuntimeError?.(error);
                }} catch {{
                    // Notification must not replace the original trap.
                }}
            }}
            throw error;
        }}
    }},
]));
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

export function createInstance(wasmModule: WebAssembly.Module, onRuntimeError?: (error: WebAssembly.RuntimeError) => void): Promise<{{
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};

    #[test]
    fn runtime_error_stops_exports_including_finalizers_before_notifying_the_host() {
        let parsed = ParsedJs {
            body_lines: vec![
                "class Editor { render() { return wasm.render(); } read() { return wasm.read(); } free() { wasm.__wbg_editor_free(); } }".into(),
                "function finalize() { wasm.__wbg_editor_free(); }".into(),
                "function __wbg_get_imports() { return {}; }".into(),
            ],
            export_names: vec!["Editor".into(), "finalize".into()],
            has_start: false,
        };
        let script = format!(
            r#"
import assert from 'node:assert/strict';
{}
// A real Wasm unreachable trap, independent of the editor's current renderer.
const trapped = new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array([
    0,97,115,109,1,0,0,0,1,4,1,96,0,0,3,2,1,0,
    7,8,1,4,116,114,97,112,0,0,10,5,1,3,0,0,11
])));
let reads = 0;
let frees = 0;
WebAssembly.instantiate = async () => ({{ exports: {{
    render: () => trapped.exports.trap(),
    read: () => ++reads,
    __wbg_editor_free: () => ++frees,
}} }});
let notified;
let notifications = 0;
const bindings = await createInstance({{}}, (error) => {{
    notified = error;
    notifications++;
    assert.throws(() => second.read(), (caught) => caught === error);
    second.free();
    bindings.finalize();
}});
const first = new bindings.Editor();
const second = new bindings.Editor();
assert.equal(second.read(), 1);
assert.equal(notifications, 0);
assert.throws(() => first.render(), (error) => error instanceof WebAssembly.RuntimeError);
assert.ok(notified instanceof WebAssembly.RuntimeError);
assert.equal(notifications, 1);
assert.throws(() => second.read(), (error) => error === notified);
assert.throws(() => first.render(), (error) => error === notified);
first.free();
bindings.finalize();
assert.equal(reads, 1);
assert.equal(frees, 0);
assert.equal(notifications, 1);
"#,
            generate_js(&parsed)
        );
        let mut child = Command::new("node")
            .arg("--input-type=module")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Node.js is required to test generated bindings");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(script.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
