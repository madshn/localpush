#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::atomic::Ordering;

use localpush_lib::{commands, setup_app, SHOULD_EXIT};

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_google_auth::init())
        .setup(|app| {
            setup_app(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_delivery_status,
            commands::get_sources,
            commands::get_delivery_queue,
            commands::enable_source,
            commands::disable_source,
            commands::add_webhook_target,
            commands::test_webhook,
            commands::get_source_preview,
            commands::get_source_sample_payload,
            commands::get_webhook_config,
            commands::get_setting,
            commands::set_setting,
            commands::retry_delivery,
            commands::connect_n8n_target,
            commands::connect_ntfy_target,
            commands::connect_make_target,
            commands::connect_zapier_target,
            commands::connect_google_sheets_target,
            commands::list_targets,
            commands::test_target_connection,
            commands::list_target_endpoints,
            commands::create_binding,
            commands::remove_binding,
            commands::get_source_bindings,
            commands::list_all_bindings,
            commands::trigger_source_push,
            commands::replay_delivery,
            commands::get_source_properties,
            commands::set_source_property,
            commands::open_feedback,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if !SHOULD_EXIT.load(Ordering::SeqCst) {
                // Keep app running in tray (window close, not explicit quit)
                api.prevent_exit();
            }
        }
    });
}
