use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: editor-bindgen-swift-types <library-path> <output-dir>");
        std::process::exit(1);
    }
    let metas = editor_bindgen::reader::read_ffi_meta(&PathBuf::from(&args[1]));
    eprintln!("Found {} FFI types", metas.len());
    editor_bindgen::swift_types::generate_all(&metas, &PathBuf::from(&args[2]));
}
