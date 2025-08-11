use tauri::Manager;

pub fn run() {
  tauri::Builder::default()
    .on_window_event(|window, event| match event {
      tauri::WindowEvent::CloseRequested { api, .. } => {
        #[cfg(target_os = "macos")]
        {
          let _ = window.hide();
          api.prevent_close();
        }
      }
      _ => {}
    })
    .plugin(tauri_plugin_single_instance::init(|app, _, _| {
      let _ = app
        .get_webview_window("main")
        .expect("no main window")
        .set_focus();
    }))
    .plugin(tauri_plugin_deep_link::init())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_http::init())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_process::init())
    .plugin(tauri_plugin_store::Builder::new().build())
    .plugin(tauri_plugin_updater::Builder::new().build())
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    .run(|app, event| match event {
      tauri::RunEvent::Reopen { .. } => {
        let window = app.get_webview_window("main").expect("no main window");
        let _ = window.show();
        let _ = window.set_focus();
      }
      _ => {}
    });
}
