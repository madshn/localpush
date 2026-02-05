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

pub mod config;
mod ledger;
mod state;
pub mod delivery_worker;

use std::sync::Arc;
use tauri::{App, Manager};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling;

pub use ledger::DeliveryLedger;
pub use state::AppState;

/// Initialize the application
pub fn setup_app(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to both stdout and file
    let log_dir = app.path().app_log_dir()?;
    std::fs::create_dir_all(&log_dir)?;
    let file_appender = rolling::daily(&log_dir, "localpush.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "localpush=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer()) // stdout
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking).with_ansi(false)) // file
        .init();

    // Keep guard alive for application lifetime
    // Store in static to prevent drop
    std::mem::forget(_guard);

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

    app.manage(state);

    // Set up system tray
    setup_tray(app)?;

    // Check for auto-update (in background, after a delay)
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        // Wait 5 seconds after startup
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Check if auto-update is enabled (default: true)
        let auto_update_enabled = match app_handle.state::<AppState>()
            .config
            .get("auto_update")
        {
            Ok(Some(value)) => value != "false",
            _ => true, // Default to enabled
        };

        if !auto_update_enabled {
            tracing::info!("Auto-update is disabled");
            return;
        }

        tracing::info!("Checking for updates...");
        // Use the updater plugin API from tauri-plugin-updater
        match tauri_plugin_updater::UpdaterExt::updater(&app_handle) {
            Ok(updater_builder) => {
                match updater_builder.check().await {
                    Ok(Some(update)) => {
                        tracing::info!(
                            current = %update.current_version,
                            latest = %update.version,
                            "Update available, downloading and installing"
                        );

                        match update.download_and_install(|_, _| {}, || {}).await {
                            Ok(()) => {
                                tracing::info!("Update installed successfully, restart required");
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "Failed to install update");
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::info!("App is up to date");
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "Update check failed (expected in dev mode)");
                    }
                }
            }
            Err(e) => {
                tracing::debug!(error = %e, "Failed to build updater (expected in dev mode)");
            }
        }
    });

    tracing::info!("LocalPush initialized — delivery pipeline active");
    Ok(())
}

fn setup_tray(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState};
    use tauri::menu::{Menu, MenuItem};

    let quit = MenuItem::with_id(app, "quit", "Quit LocalPush", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit])?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png"))?;

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "quit" {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    // Toggle: hide if visible, show if hidden
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                        return;
                    }

                    // Extract tray icon position (may be physical or logical)
                    let scale = window.scale_factor().unwrap_or(2.0);
                    let (icon_x, icon_y) = match rect.position {
                        tauri::Position::Physical(p) => (p.x as f64 / scale, p.y as f64 / scale),
                        tauri::Position::Logical(p) => (p.x, p.y),
                    };
                    let (icon_w, icon_h) = match rect.size {
                        tauri::Size::Physical(s) => (s.width as f64 / scale, s.height as f64 / scale),
                        tauri::Size::Logical(s) => (s.width, s.height),
                    };

                    // Position window centered below the tray icon
                    let window_width = 400.0_f64;
                    let window_height = 500.0_f64;
                    let x = icon_x + (icon_w / 2.0) - (window_width / 2.0);
                    let y = icon_y + icon_h + 4.0;

                    let _ = window.set_position(tauri::Position::Logical(
                        tauri::LogicalPosition::new(x, y),
                    ));
                    let _ = window.set_size(tauri::Size::Logical(
                        tauri::LogicalSize::new(window_width, window_height),
                    ));
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    // Hide window when it loses focus (click outside to dismiss)
    if let Some(window) = app.get_webview_window("main") {
        let win = window.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::Focused(false) = event {
                let _ = win.hide();
            }
        });
    }

    Ok(())
}
