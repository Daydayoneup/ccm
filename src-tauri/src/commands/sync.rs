use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use crate::db::Database;
use crate::sync::{SyncEngine, SyncReport};
use crate::sync::state::SyncState;
use crate::scanner;
use crate::models::v2::{Plugin, ResourceScope};
use crate::commands::registry::upsert_registry_plugins;

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

#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct SyncImpact {
    pub updates_available: usize,
    pub upstream_removed: usize,
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
    emit_progress(app, "global", 1, 5, "Scanning global resources...");
    let adapter_registry = crate::adapters::AdapterRegistry::new();
    let home = dirs::home_dir().unwrap_or_default();
    let global_scanned = scanner::scan_resources_for_sync(
        &home.join(".claude"),
        &ResourceScope::Global,
        &adapter_registry,
    );
    accumulate(
        &mut total_report,
        &mut errors,
        SyncEngine::reconcile(db, &ResourceScope::Global, global_scanned),
    );

    // Stage 2: Library resources
    emit_progress(app, "library", 2, 5, "Scanning library resources...");
    let library_dir = home.join(".claude-manager").join("library");
    if library_dir.is_dir() {
        let library_scanned = scanner::scan_resources_for_sync(
            &library_dir,
            &ResourceScope::Library,
            &adapter_registry,
        );
        accumulate(
            &mut total_report,
            &mut errors,
            SyncEngine::reconcile(db, &ResourceScope::Library, library_scanned),
        );
    }

    // Stage 3: Project resources (adapter-based, includes MCP)
    emit_progress(app, "projects", 3, 5, "Scanning project resources...");
    let mut all_project_scanned: Vec<scanner::ScannedResource> = Vec::new();
    match db.list_projects() {
        Ok(projects) => {
            let project_count = projects.len();
            for (i, project) in projects.iter().enumerate() {
                emit_progress(
                    app,
                    "projects",
                    3,
                    5,
                    &format!("Scanning project {}/{}: {}", i + 1, project_count, project.name),
                );
                let project_path = std::path::Path::new(&project.path);
                all_project_scanned.extend(scanner::scan_resources_for_sync(
                    project_path,
                    &ResourceScope::Project,
                    &adapter_registry,
                ));
            }
        }
        Err(e) => {
            let msg = format!("Failed to list projects: {}", e);
            eprintln!("{}", msg);
            errors.push(msg);
        }
    }
    accumulate(
        &mut total_report,
        &mut errors,
        SyncEngine::reconcile(db, &ResourceScope::Project, all_project_scanned),
    );

    // Stage 4: Plugins
    emit_progress(app, "plugins", 4, 5, "Scanning installed plugins...");
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

    // Stage 5: Registries
    emit_progress(app, "registries", 5, 5, "Scanning registries...");
    match db.list_registries() {
        Ok(registries) => {
            for registry in &registries {
                if let Err(e) = upsert_registry_plugins(db, registry) {
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

    // Compute sync impact: how many installed resources have updates or were removed
    let mut sync_impact = SyncImpact::default();
    if let Ok(all_links) = db.list_all_links() {
        let mut seen_resources: std::collections::HashSet<String> = std::collections::HashSet::new();
        for link in &all_links {
            if link.link_type != "symlink" || link.installed_hash.is_none() {
                continue;
            }
            if !seen_resources.insert(link.resource_id.clone()) {
                continue; // already counted
            }
            if let Ok(Some(resource)) = db.get_resource(&link.resource_id) {
                if resource.is_draft == -1 {
                    sync_impact.upstream_removed += 1;
                } else if let Some(ref content_hash) = resource.content_hash {
                    if link.installed_hash.as_deref() != Some(content_hash.as_str()) {
                        sync_impact.updates_available += 1;
                    }
                }
            }
        }
    }
    let _ = app.emit("sync-impact", &sync_impact);

    emit_progress(app, "done", 5, 5, "Sync complete");
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
    let adapter_registry = crate::adapters::AdapterRegistry::new();

    let scanned = match resource_scope {
        ResourceScope::Global => {
            let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
            scanner::scan_resources_for_sync(
                &home.join(".claude"),
                &resource_scope,
                &adapter_registry,
            )
        }
        ResourceScope::Library => {
            let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
            let library_dir = home.join(".claude-manager").join("library");
            if library_dir.is_dir() {
                scanner::scan_resources_for_sync(&library_dir, &resource_scope, &adapter_registry)
            } else {
                Vec::new()
            }
        }
        ResourceScope::Project => {
            let projects = db.list_projects().map_err(|e| e.to_string())?;
            let mut all = Vec::new();
            for project in &projects {
                let project_path = std::path::Path::new(&project.path);
                all.extend(scanner::scan_resources_for_sync(
                    project_path,
                    &resource_scope,
                    &adapter_registry,
                ));
            }
            all
        }
        ResourceScope::Plugin => {
            let scanned_plugins = scanner::plugin::scan_installed_plugins();
            scanned_plugins
                .into_iter()
                .flat_map(|sp| sp.resources)
                .collect()
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
