pub mod adapters;
pub mod commands;
pub mod db;
pub mod frontmatter;
pub mod models;
pub mod git;
pub mod proxy;
pub mod scanner;
pub mod sync;
pub mod http;

use db::Database;
use sync::watcher::FsWatcher;
use tauri::{Manager, Emitter};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let home = dirs::home_dir().unwrap_or_default();
    let db_path = home.join(".claude-manager").join("ccm.db");
    let database = Database::new(db_path.to_str().unwrap())
        .expect("Failed to initialize database");

    let (shutdown_tx, _shutdown_rx) = tokio::sync::watch::channel(false);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(database)
        .manage(adapters::AdapterRegistry::new())
        .manage(shutdown_tx)
        .setup(move |app| {
            // Set macOS dock icon (needed for dev mode)
            #[cfg(target_os = "macos")]
            {
                use objc2::{AnyThread, MainThreadMarker};
                use objc2_app_kit::{NSApplication, NSImage};
                use objc2_foundation::NSData;
                let icon_data = include_bytes!("../icons/icon.png");
                let data = NSData::with_bytes(icon_data);
                if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
                    let mtm = MainThreadMarker::new().unwrap();
                    let ns_app = NSApplication::sharedApplication(mtm);
                    unsafe { ns_app.setApplicationIconImage(Some(&image)) };
                }
            }

            let app_handle = app.handle().clone();

            // Run JSON migration synchronously (one-time operation)
            let db = app.state::<Database>().inner().clone();
            if let Err(e) = db::migration::migrate_json_to_sqlite(&db) {
                eprintln!("JSON migration warning: {}", e);
            }

            db.migrate_v2_to_v3().map_err(|e| format!("Schema migration failed: {}", e))?;
            db.migrate_v3_to_v4().map_err(|e| format!("Schema migration v3→v4 failed: {}", e))?;
            db.migrate_v4_to_v5().ok();

            // Register SyncState wrapped in Arc for thread sharing
            app.manage(std::sync::Arc::new(sync::state::SyncState::new()));

            // Start FS watcher
            let watched_paths = vec![
                home.join(".claude"),
                home.join(".claude-manager"),
            ];

            let handle = app_handle.clone();
            match FsWatcher::new(watched_paths, move |changed_paths| {
                let paths: Vec<String> = changed_paths
                    .iter()
                    .filter_map(|p| p.to_str().map(|s| s.to_string()))
                    .collect();
                let _ = handle.emit("fs-change", paths);
            }) {
                Ok(watcher) => {
                    app.manage(watcher);
                }
                Err(e) => {
                    eprintln!("Failed to start FS watcher: {}", e);
                }
            }

            // --- HTTP API server ---
            let db_for_http = std::sync::Arc::new(
                app.state::<Database>().inner().clone()
            );

            let api_enabled = db_for_http.get_setting("api_enabled")
                .unwrap_or(None)
                .unwrap_or_default() == "true";

            if api_enabled {
                let port: u16 = db_for_http.get_setting("api_port")
                    .unwrap_or(None)
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(23890);

                let shutdown_tx_ref = app.state::<tokio::sync::watch::Sender<bool>>();
                let shutdown_rx = shutdown_tx_ref.subscribe();
                let db_clone = db_for_http.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = crate::http::start_server(db_clone, port, shutdown_rx).await {
                        eprintln!("HTTP API server error: {}", e);
                    }
                });
            }

            // --- System tray ---
            use tauri::tray::TrayIconBuilder;
            use tauri::menu::{MenuBuilder, MenuItemBuilder};

            let show_item = MenuItemBuilder::with_id("show", "Show Window").build(app)?;
            let api_label = if api_enabled { "API: Running" } else { "API: Disabled" };
            let api_item = MenuItemBuilder::with_id("api_status", api_label)
                .enabled(false)
                .build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&api_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().expect("default window icon not set in tauri.conf.json").clone())
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            // Restore dock icon when window is shown
                            #[cfg(target_os = "macos")]
                            {
                                use objc2::MainThreadMarker;
                                use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
                                if let Some(mtm) = MainThreadMarker::new() {
                                    let ns_app = NSApplication::sharedApplication(mtm);
                                    ns_app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
                                }
                            }
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Intercept window close → hide to tray + hide dock icon
            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w.hide();
                        // Hide dock icon when window is hidden
                        #[cfg(target_os = "macos")]
                        {
                            use objc2::MainThreadMarker;
                            use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
                            if let Some(mtm) = MainThreadMarker::new() {
                                let ns_app = NSApplication::sharedApplication(mtm);
                                ns_app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::symlinks::link_resource,
            commands::symlinks::unlink_resource,
            commands::symlinks::is_symlink_valid,
            commands::files::read_file,
            commands::files::write_file,
            commands::files::delete_path,
            commands::files::create_directory,
            commands::files::list_directory,
            commands::files::file_content_hash,
            commands::files::rename_path,
            commands::migration::import_resources_from_project,
            commands::migration::scan_global_resources,
            commands::migration::import_global_resources,
            commands::migration::restore_global_resources,
            commands::migration::scan_installed_plugins,
            commands::migration::scan_mcp_configs,
            commands::sync::full_sync,
            commands::sync::sync_scope,
            commands::sync::get_dashboard_stats,
            commands::sync::get_recent_resources,
            commands::sync::search_resources,
            commands::global::list_global_resources,
            commands::global::create_global_resource,
            commands::global::delete_global_resource,
            commands::global::backup_to_library,
            commands::projects_v2::list_projects_v2,
            commands::projects_v2::register_project_v2,
            commands::projects_v2::remove_project_v2,
            commands::projects_v2::rescan_project,
            commands::projects_v2::discover_claude_projects,
            commands::projects_v2::scan_and_discover_projects,
            commands::projects_v2::list_project_resources,
            commands::projects_v2::create_project_resource,
            commands::projects_v2::delete_project_resource,
            commands::projects_v2::publish_to_library,
            commands::projects_v2::install_from_library,
            commands::projects_v2::list_project_mcp_servers,
            commands::projects_v2::list_global_mcp_servers,
            commands::projects_v2::get_project_permissions,
            commands::projects_v2::update_project_permissions,
            commands::projects_v2::toggle_project_pin,
            commands::projects_v2::list_projects_ranked,
            commands::settings::get_app_setting,
            commands::settings::set_app_setting,
            commands::plugins_v2::list_plugins_v2,
            commands::plugins_v2::scan_plugins,
            commands::plugins_v2::get_plugin_resources,
            commands::plugins_v2::extract_to_library,
            commands::plugins_v2::install_plugin,
            commands::plugins_v2::uninstall_plugin,
            commands::library_v2::list_library_resources,
            commands::library_v2::create_library_resource,
            commands::library_v2::delete_library_resource,
            commands::library_v2::install_to_project,
            commands::library_v2::deploy_to_global,
            commands::library_v2::list_resource_links,
            commands::library_v2::check_link_health,
            commands::library_v2::fork_to_library,
            commands::shell::launch_claude_in_terminal,
            commands::shell::get_terminal_preference,
            commands::shell::set_terminal_preference,
            commands::env::list_env_vars,
            commands::env::set_env_var,
            commands::env::delete_env_var,
            commands::env::list_merged_env_vars,
            commands::registry::list_registries,
            commands::registry::add_registry,
            commands::registry::remove_registry,
            commands::registry::sync_registry,
            commands::registry::sync_all_registries,
            commands::registry::push_registry,
            commands::registry::check_registry_updates,
            commands::registry::list_registry_resources,
            commands::registry::publish_to_registry,
            commands::registry::install_from_registry,
            commands::registry::deploy_from_registry,
            commands::registry::list_registry_plugins,
            commands::registry::get_registry_plugin_resources,
            commands::registry::get_registry_plugin_mcp_servers,
            commands::registry::install_plugin_to_project,
            commands::registry::install_plugin_to_global,
            commands::registry::uninstall_plugin_from_project,
            commands::registry::install_resource_to_project,
            commands::registry::install_resource_to_global,
            commands::registry::uninstall_resource,
            commands::registry::get_plugin_resources_install_status,
            commands::library_plugin::create_library_plugin,
            commands::library_plugin::delete_library_plugin,
            commands::library_plugin::list_library_plugins,
            commands::library_plugin::add_resource_to_library_plugin,
            commands::library_plugin::remove_resource_from_library_plugin,
            commands::library_plugin::get_library_plugin_resources,
            commands::proxy::get_proxy_config,
            commands::proxy::save_proxy_config,
            commands::proxy::test_proxy,
            commands::api::generate_api_token,
            commands::api::get_api_token_status,
            commands::api::toggle_api_server,
            commands::versions::publish_resource_version,
            commands::versions::list_resource_versions,
            commands::versions::rollback_resource_version,
            commands::frontmatter_cmd::parse_skill_frontmatter,
            commands::frontmatter_cmd::save_skill_with_frontmatter,
            commands::frontmatter_cmd::save_skill_raw_content,
            commands::frontmatter_cmd::get_resource,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            match &event {
                tauri::RunEvent::ExitRequested { code, api, .. } => {
                    // Prevent auto-exit when all windows are hidden (tray mode)
                    if code.is_none() {
                        api.prevent_exit();
                    }
                }
                #[cfg(target_os = "macos")]
                tauri::RunEvent::Reopen { has_visible_windows, .. } => {
                    if !has_visible_windows {
                        // Restore dock icon
                        use objc2::MainThreadMarker;
                        use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
                        if let Some(mtm) = MainThreadMarker::new() {
                            let ns_app = NSApplication::sharedApplication(mtm);
                            ns_app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
                        }
                        // Show and focus the main window
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
                _ => {}
            }
            let _ = app;
        });
}
