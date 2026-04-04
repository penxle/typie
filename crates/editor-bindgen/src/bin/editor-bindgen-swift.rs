use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: swift <library-path> <output-dir>");
        std::process::exit(1);
    }

    let library_path = PathBuf::from(&args[1]);
    let output_dir = PathBuf::from(&args[2]);

    let metas = editor_bindgen::reader::read_ffi_meta(&library_path);
    let custom_types: std::collections::HashMap<String, String> = metas
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

    editor_bindgen::swift::generate_all(&interfaces, &custom_types, &output_dir);
    eprintln!("Generated Swift files in {}", output_dir.display());
}
