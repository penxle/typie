fn main() {
    env_logger::init();
    if let Err(e) = wasm_bindgen_cli::wasm_bindgen::run_cli_with_args(std::env::args_os()) {
        eprintln!("error: {e:?}");
        std::process::exit(1);
    }
}
