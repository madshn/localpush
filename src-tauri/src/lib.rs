//! LocalPush - Push local file changes to webhooks with guaranteed delivery
//!
//! This library provides the core functionality for LocalPush, organized around
//! trait-based dependency injection for testability.

pub mod commands;
pub mod traits;
pub mod mocks;
pub mod production;
pub mod sources;

mod ledger;
mod state;

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
    app.manage(Arc::new(state));

    // Set up system tray
    setup_tray(app)?;

    tracing::info!("LocalPush initialized");
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
