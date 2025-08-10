use tauri::Manager;

pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_single_instance::init(|app, _, _| {
      let _ = app
        .get_webview_window("main")
        .expect("no main window")
        .set_focus();
    }))
    .plugin(tauri_plugin_deep_link::init())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
