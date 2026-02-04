//! LocalPush - Push local file changes to webhooks with guaranteed delivery
//!
//! This library provides the core functionality for LocalPush, organized around
//! trait-based dependency injection for testability.

pub mod commands;
pub mod traits;
pub mod mocks;
pub mod production;
pub mod sources;
pub mod source_manager;

mod config;
mod ledger;
mod state;
pub mod delivery_worker;

use std::sync::Arc;
use tauri::{App, Manager};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub use ledger::DeliveryLedger;
pub use state::AppState;

/// Initialize the application
pub fn setup_app(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "localpush=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("LocalPush starting up");

    // Initialize app state with production implementations
    let state = AppState::new_production(app.handle())?;

    // Recover orphaned in-flight entries from previous crash
    let recovered = state.ledger.recover_orphans().unwrap_or(0);
    if recovered > 0 {
        tracing::warn!("Recovered {} orphaned deliveries from previous session", recovered);
    }

    // Connect file watcher events to source manager
    let source_manager_for_events = state.source_manager.clone();
    state.file_watcher.set_event_handler(Arc::new(move |event| {
        tracing::debug!("File event: {:?}", event.path);
        if let Err(e) = source_manager_for_events.handle_file_event(&event.path) {
            tracing::warn!("Failed to process file event {:?}: {}", event.path, e);
        }
    }));

    // Spawn background delivery worker
    let _worker = delivery_worker::spawn_worker(
        state.ledger.clone(),
        state.webhook_client.clone(),
        state.config.clone(),
    );

    let state = Arc::new(state);
    app.manage(state);

    // Set up system tray
    setup_tray(app)?;

    tracing::info!("LocalPush initialized — delivery pipeline active");
    Ok(())
}

fn setup_tray(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState};
    use tauri::menu::{Menu, MenuItem};

    let quit = MenuItem::with_id(app, "quit", "Quit LocalPush", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit])?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "quit" {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
