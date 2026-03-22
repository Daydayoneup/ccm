use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use crate::db::Database;
use crate::sync::{SyncEngine, SyncReport};
use crate::sync::state::SyncState;
use crate::scanner;
use crate::models::v2::{Plugin, ResourceScope, McpServer};
use crate::commands::registry::scan_and_insert_plugins;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncCommandResult {
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncProgress {
    pub stage: String,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

fn emit_progress(app: &AppHandle, stage: &str, current: usize, total: usize, message: &str) {
    let progress = SyncProgress {
        stage: stage.to_string(),
        current,
        total,
        message: message.to_string(),
    };
    let _ = app.emit("sync-progress", progress);
}

fn accumulate(total: &mut SyncReport, errors: &mut Vec<String>, result: Result<SyncReport, String>) {
    match result {
        Ok(report) => {
            total.inserted += report.inserted;
            total.updated += report.updated;
            total.deleted += report.deleted;
        }
        Err(e) => {
            eprintln!("Sync stage error: {}", e);
            errors.push(e);
        }
    }
}

fn run_sync(app: &AppHandle, db: &Database) -> (SyncReport, Vec<String>) {
    let mut total_report = SyncReport::default();
    let mut errors: Vec<String> = Vec::new();

    // Stage 1: Global resources
    emit_progress(app, "global", 1, 6, "Scanning global resources...");
    let global_scanned = scanner::global::scan_global_resources();
    accumulate(
        &mut total_report,
        &mut errors,
        SyncEngine::reconcile(db, &ResourceScope::Global, global_scanned),
    );

    // Stage 2: Library resources
    emit_progress(app, "library", 2, 6, "Scanning library resources...");
    let home = dirs::home_dir().unwrap_or_default();
    let library_dir = home.join(".claude-manager").join("library");
    if library_dir.is_dir() {
        let local = scanner::scan_claude_dir(&library_dir);
        let library_scanned: Vec<scanner::ScannedResource> = local
            .into_iter()
            .map(|lr| {
                let hash = scanner::compute_file_hash(&lr.path);
                scanner::ScannedResource {
                    resource_type: scanner::v1_to_v2_resource_type(&lr.resource_type),
                    name: lr.name,
                    source_path: lr.path,
                    content_hash: hash,
                }
            })
            .collect();
        accumulate(
            &mut total_report,
            &mut errors,
            SyncEngine::reconcile(db, &ResourceScope::Library, library_scanned),
        );
    }

    // Stage 3: Project resources
    emit_progress(app, "projects", 3, 6, "Scanning project resources...");
    match db.list_projects() {
        Ok(projects) => {
            let project_count = projects.len();
            for (i, project) in projects.iter().enumerate() {
                emit_progress(
                    app,
                    "projects",
                    3,
                    6,
                    &format!("Scanning project {}/{}: {}", i + 1, project_count, project.name),
                );
                let project_path = std::path::Path::new(&project.path);
                let claude_dir = project_path.join(".claude");
                if claude_dir.is_dir() {
                    let local = scanner::scan_claude_dir(&claude_dir);
                    let project_scanned: Vec<scanner::ScannedResource> = local
                        .into_iter()
                        .map(|lr| {
                            let hash = scanner::compute_file_hash(&lr.path);
                            scanner::ScannedResource {
                                resource_type: scanner::v1_to_v2_resource_type(&lr.resource_type),
                                name: lr.name,
                                source_path: lr.path,
                                content_hash: hash,
                            }
                        })
                        .collect();
                    accumulate(
                        &mut total_report,
                        &mut errors,
                        SyncEngine::reconcile(db, &ResourceScope::Project, project_scanned),
                    );
                }
            }
        }
        Err(e) => {
            let msg = format!("Failed to list projects: {}", e);
            eprintln!("{}", msg);
            errors.push(msg);
        }
    }

    // Stage 4: Plugins
    emit_progress(app, "plugins", 4, 6, "Scanning installed plugins...");
    let scanned_plugins = scanner::plugin::scan_installed_plugins();
    match db.list_plugins() {
        Ok(existing_plugins) => {
            let now = chrono::Utc::now().to_rfc3339();

            for sp in &scanned_plugins {
                let existing = existing_plugins.iter().find(|p| {
                    p.install_path.as_deref() == Some(&sp.install_path)
                });

                if let Some(existing_plugin) = existing {
                    let updated = Plugin {
                        id: existing_plugin.id.clone(),
                        name: sp.name.clone(),
                        version: if sp.version.is_empty() { None } else { Some(sp.version.clone()) },
                        scope: if sp.scope.is_empty() { None } else { Some(sp.scope.clone()) },
                        install_path: Some(sp.install_path.clone()),
                        status: "installed".to_string(),
                        last_checked: Some(now.clone()),
                    };
                    let _ = db.update_plugin(&updated);
                } else {
                    let plugin = Plugin {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: sp.name.clone(),
                        version: if sp.version.is_empty() { None } else { Some(sp.version.clone()) },
                        scope: if sp.scope.is_empty() { None } else { Some(sp.scope.clone()) },
                        install_path: Some(sp.install_path.clone()),
                        status: "installed".to_string(),
                        last_checked: Some(now.clone()),
                    };
                    let _ = db.insert_plugin(&plugin);
                }
            }

            // Remove plugins from DB that are no longer installed
            for existing in &existing_plugins {
                let still_installed = scanned_plugins.iter().any(|sp| {
                    existing.install_path.as_deref() == Some(&sp.install_path)
                });
                if !still_installed {
                    let _ = db.delete_plugin(&existing.id);
                }
            }
        }
        Err(e) => {
            let msg = format!("Failed to list plugins: {}", e);
            eprintln!("{}", msg);
            errors.push(msg);
        }
    }

    // Sync plugin resources
    let mut plugin_resources = Vec::new();
    for sp in &scanned_plugins {
        plugin_resources.extend(sp.resources.clone());
    }
    accumulate(
        &mut total_report,
        &mut errors,
        SyncEngine::reconcile(db, &ResourceScope::Plugin, plugin_resources),
    );

    // Stage 5: MCP servers
    emit_progress(app, "mcp", 5, 6, "Scanning MCP servers...");
    if let Ok(old_servers) = db.list_global_mcp_servers() {
        for s in old_servers {
            let _ = db.delete_mcp_server(&s.id);
        }
    }
    let mut global_mcp = scanner::mcp::scan_global_mcp();
    global_mcp.extend(scanner::mcp::scan_plugin_mcp_servers());
    for scanned in global_mcp {
        let server = McpServer {
            id: uuid::Uuid::new_v4().to_string(),
            name: scanned.name,
            project_id: None,
            server_type: scanned.server_type,
            command: scanned.command,
            args: scanned.args,
            url: scanned.url,
            env: scanned.env,
            source_path: scanned.source_path,
            registry_plugin_id: None,
        };
        let _ = db.insert_mcp_server(&server);
    }

    // Stage 6: Registries
    emit_progress(app, "registries", 6, 6, "Scanning registries...");
    match db.list_registries() {
        Ok(registries) => {
            for registry in &registries {
                // Delete old plugins and resources
                if let Ok(old_plugins) = db.list_registry_plugins(&registry.id) {
                    if let Ok(resources) = db.list_resources_by_scope(&ResourceScope::Registry) {
                        for old_plugin in &old_plugins {
                            for r in &resources {
                                if r.metadata.as_deref() == Some(&old_plugin.id) {
                                    let _ = db.delete_resource(&r.id);
                                }
                            }
                        }
                    }
                }
                let _ = db.delete_registry_plugins_by_registry(&registry.id);

                if let Err(e) = scan_and_insert_plugins(db, registry) {
                    let msg = format!("Failed to scan registry {}: {}", registry.name, e);
                    eprintln!("{}", msg);
                    errors.push(msg);
                }
            }
        }
        Err(e) => {
            let msg = format!("Failed to list registries: {}", e);
            eprintln!("{}", msg);
            errors.push(msg);
        }
    }

    emit_progress(app, "done", 6, 6, "Sync complete");
    (total_report, errors)
}

struct SyncIdleGuard(Arc<SyncState>);

impl Drop for SyncIdleGuard {
    fn drop(&mut self) {
        self.0.set_idle();
    }
}

#[tauri::command]
pub async fn full_sync(
    db: State<'_, Database>,
    sync_state: State<'_, Arc<SyncState>>,
    app: AppHandle,
) -> Result<SyncCommandResult, String> {
    let state = sync_state.inner().clone();

    if !state.try_start() {
        state.set_pending();
        return Ok(SyncCommandResult { status: "queued".to_string() });
    }

    let db_clone = db.inner().clone();
    let state_clone = state.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let _guard = SyncIdleGuard(state_clone.clone());

        loop {
            let (report, errors) = run_sync(&app_clone, &db_clone);

            // Always emit sync-complete with the report (partial success preserved)
            let _ = app_clone.emit("sync-complete", &report);

            // Emit sync-error if any stages failed
            if !errors.is_empty() {
                let _ = app_clone.emit("sync-error", errors.join("; "));
            }

            // Check if a re-run was requested while we were running
            if !state_clone.take_pending() {
                break;
            }

            // Re-run requested — keep the guard alive (state stays Running)
        }
        // _guard drops here, calling set_idle()
    });

    Ok(SyncCommandResult { status: "started".to_string() })
}

#[tauri::command]
pub fn sync_scope(db: State<Database>, scope: String) -> Result<SyncReport, String> {
    let resource_scope =
        ResourceScope::from_str(&scope).ok_or_else(|| format!("Invalid scope: {}", scope))?;

    let scanned = match resource_scope {
        ResourceScope::Global => scanner::global::scan_global_resources(),
        ResourceScope::Library => {
            let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
            let library_dir = home.join(".claude-manager").join("library");
            if library_dir.is_dir() {
                let local = scanner::scan_claude_dir(&library_dir);
                local
                    .into_iter()
                    .map(|lr| {
                        let hash = scanner::compute_file_hash(&lr.path);
                        scanner::ScannedResource {
                            resource_type: scanner::v1_to_v2_resource_type(&lr.resource_type),
                            name: lr.name,
                            source_path: lr.path,
                            content_hash: hash,
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            }
        }
        ResourceScope::Plugin => {
            let scanned_plugins = scanner::plugin::scan_installed_plugins();
            scanned_plugins
                .into_iter()
                .flat_map(|sp| sp.resources)
                .collect()
        }
        ResourceScope::Project => {
            // For project scope, sync all registered projects
            let projects = db.list_projects().map_err(|e| e.to_string())?;
            let mut all = Vec::new();
            for project in &projects {
                let claude_dir = std::path::Path::new(&project.path).join(".claude");
                if claude_dir.is_dir() {
                    let local = scanner::scan_claude_dir(&claude_dir);
                    all.extend(local.into_iter().map(|lr| {
                        let hash = scanner::compute_file_hash(&lr.path);
                        scanner::ScannedResource {
                            resource_type: scanner::v1_to_v2_resource_type(&lr.resource_type),
                            name: lr.name,
                            source_path: lr.path,
                            content_hash: hash,
                        }
                    }));
                }
            }
            all
        }
        ResourceScope::Registry => {
            // Registry sync is handled separately via registry-specific commands
            Vec::new()
        }
    };

    SyncEngine::reconcile(&db, &resource_scope, scanned)
}

#[tauri::command]
pub fn get_dashboard_stats(
    db: State<Database>,
) -> Result<crate::models::v2::DashboardStats, String> {
    let global_count = db
        .count_resources_by_scope(&ResourceScope::Global)
        .map_err(|e| e.to_string())?;
    let library_count = db
        .count_resources_by_scope(&ResourceScope::Library)
        .map_err(|e| e.to_string())?;
    let project_count = db.count_projects().map_err(|e| e.to_string())?;
    let plugin_count = db.count_plugins().map_err(|e| e.to_string())?;
    let registry_count = db.count_registries().map_err(|e| e.to_string())?;

    Ok(crate::models::v2::DashboardStats {
        global_count,
        project_count,
        plugin_count,
        library_count,
        registry_count,
    })
}

#[tauri::command]
pub fn get_recent_resources(
    db: State<Database>,
    limit: usize,
) -> Result<Vec<crate::models::v2::Resource>, String> {
    db.list_recent_resources(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_resources(
    db: State<Database>,
    query: String,
) -> Result<Vec<crate::models::v2::Resource>, String> {
    db.search_resources(&query).map_err(|e| e.to_string())
}
