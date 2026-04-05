use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: kotlin <library-path> --base-dir <dir>");
        std::process::exit(1);
    }

    let library_path = PathBuf::from(&args[1]);
    let base_dir = args
        .iter()
        .position(|a| a == "--base-dir")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            eprintln!("Error: --base-dir required");
            std::process::exit(1);
        });

    let metas = editor_bindgen::reader::read_ffi_meta(&library_path);
    eprintln!("Found {} FFI types", metas.len());

    let custom_types: HashMap<String, String> = metas
        .iter()
        .filter_map(|m| match &m.kind {
            editor_bindgen::meta::FfiKind::Custom { target } => {
                Some((m.name.clone(), target.clone()))
            }
            _ => None,
        })
        .collect();

    let interfaces = editor_bindgen::reader::read_ffi_interfaces(&library_path);
    eprintln!("Found {} FFI interfaces", interfaces.len());

    let common = base_dir.join("commonMain");
    let jna = base_dir.join("jnaMain");
    let ios = base_dir.join("iosMain");

    editor_bindgen::kotlin::generate_all(&metas, &common);
    editor_bindgen::kotlin_iface::generate_all(&interfaces, &custom_types, &common);
    editor_bindgen::kotlin_jna::generate_all(&interfaces, &custom_types, &jna);
    editor_bindgen::kotlin_ios::generate_all(&interfaces, &custom_types, &ios);

    eprintln!("Generated Kotlin files");
}
