#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use localpush_lib::{commands, setup_app};

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            setup_app(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_delivery_status,
            commands::get_sources,
            commands::get_delivery_queue,
            commands::enable_source,
            commands::disable_source,
            commands::add_webhook_target,
            commands::test_webhook,
            commands::get_source_preview,
            commands::get_webhook_config,
            commands::get_setting,
            commands::set_setting,
            commands::retry_delivery,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            // Keep app running in tray
            api.prevent_exit();
        }
    });
}
