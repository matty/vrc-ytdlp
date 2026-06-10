mod commands;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::app::version,
            commands::app::default_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
