use rusqlite::{Connection, Result};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub mod resources;
pub mod projects;
pub mod plugins;
pub mod links;
pub mod sync_state;
pub mod migration;
pub mod env;
pub mod registry;
pub mod library_plugin;
pub mod versions;

#[derive(Clone)]
pub struct Database {
    pub conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let path = Path::new(db_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_e| {
                rusqlite::Error::InvalidPath(parent.to_path_buf())
            })?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn: Arc::new(Mutex::new(conn)) };
        db.init_tables()?;
        Ok(db)
    }

    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let db = Self { conn: Arc::new(Mutex::new(conn)) };
        db.init_tables()?;
        Ok(db)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query(rusqlite::params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS resources (
                id              TEXT PRIMARY KEY,
                resource_type   TEXT NOT NULL,
                name            TEXT NOT NULL,
                description     TEXT,
                scope           TEXT NOT NULL,
                source_path     TEXT NOT NULL,
                content_hash    TEXT,
                metadata        TEXT,
                created_at      TEXT NOT NULL,
                updated_at      TEXT NOT NULL,
                version         TEXT,
                is_draft        INTEGER DEFAULT 1,
                installed_from_id TEXT
            );
            CREATE TABLE IF NOT EXISTS resource_versions (
                id              TEXT PRIMARY KEY,
                resource_id     TEXT NOT NULL,
                version         TEXT NOT NULL,
                changelog       TEXT,
                content_hash    TEXT NOT NULL,
                created_at      TEXT NOT NULL,
                FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE CASCADE,
                UNIQUE(resource_id, version)
            );
            CREATE TABLE IF NOT EXISTS projects (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                path            TEXT UNIQUE NOT NULL,
                language        TEXT,
                last_scanned    TEXT,
                pinned          INTEGER DEFAULT 0,
                launch_count    INTEGER DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS plugins (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                version         TEXT,
                scope           TEXT,
                install_path    TEXT,
                status          TEXT NOT NULL,
                last_checked    TEXT
            );
            CREATE TABLE IF NOT EXISTS resource_links (
                id              TEXT PRIMARY KEY,
                resource_id     TEXT NOT NULL,
                target_scope    TEXT NOT NULL,
                target_path     TEXT NOT NULL,
                config_key      TEXT,
                project_id      TEXT,
                link_type       TEXT NOT NULL,
                created_at      TEXT NOT NULL,
                installed_hash  TEXT,
                FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE CASCADE,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS sync_state (
                id              TEXT PRIMARY KEY,
                watched_path    TEXT NOT NULL,
                last_hash       TEXT,
                last_synced     TEXT,
                status          TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS app_settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS claude_env_vars (
                id         TEXT PRIMARY KEY,
                project_id TEXT,
                key        TEXT NOT NULL,
                value      TEXT NOT NULL,
                UNIQUE(project_id, key),
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS registries (
                id                  TEXT PRIMARY KEY,
                name                TEXT NOT NULL,
                url                 TEXT UNIQUE NOT NULL,
                local_path          TEXT NOT NULL,
                readonly            INTEGER NOT NULL DEFAULT 1,
                last_synced         TEXT,
                has_remote_changes  INTEGER NOT NULL DEFAULT 0,
                has_local_changes   INTEGER NOT NULL DEFAULT 0,
                created_at          TEXT NOT NULL
            );"
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS registry_plugins (
                id TEXT PRIMARY KEY,
                registry_id TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                category TEXT,
                source_path TEXT NOT NULL,
                source_type TEXT NOT NULL DEFAULT 'local',
                source_url TEXT,
                homepage TEXT,
                FOREIGN KEY (registry_id) REFERENCES registries(id) ON DELETE CASCADE
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS library_plugins (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                category TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS library_plugin_resources (
                id TEXT PRIMARY KEY,
                plugin_id TEXT NOT NULL,
                resource_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (plugin_id) REFERENCES library_plugins(id) ON DELETE CASCADE,
                FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE CASCADE,
                UNIQUE(plugin_id, resource_id)
            )",
            [],
        )?;
        Ok(())
    }

    pub fn migrate_v2_to_v3(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // Check current version
        let current_version: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .ok();

        let ver = current_version.as_deref().and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
        if ver >= 3 {
            return Ok(());
        }

        // Add config_key column if not exists
        let has_config_key: bool = conn
            .prepare("SELECT config_key FROM resource_links LIMIT 0")
            .is_ok();

        if !has_config_key {
            conn.execute("ALTER TABLE resource_links ADD COLUMN config_key TEXT", [])
                .map_err(|e| e.to_string())?;
        }

        // Migrate mcp_servers rows to resources table (if the table still exists)
        let has_mcp_table: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='mcp_servers'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        if has_mcp_table > 0 {
            let mut stmt = conn.prepare(
                "SELECT id, name, project_id, server_type, command, args, url, env, source_path, registry_plugin_id FROM mcp_servers"
            ).map_err(|e| e.to_string())?;

            let servers: Vec<_> = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,              // id
                    row.get::<_, String>(1)?,              // name
                    row.get::<_, Option<String>>(2)?,      // project_id
                    row.get::<_, Option<String>>(3)?,      // server_type
                    row.get::<_, Option<String>>(4)?,      // command
                    row.get::<_, Option<String>>(5)?,      // args
                    row.get::<_, Option<String>>(6)?,      // url
                    row.get::<_, Option<String>>(7)?,      // env
                    row.get::<_, String>(8)?,              // source_path
                    row.get::<_, Option<String>>(9)?,      // registry_plugin_id
                ))
            }).map_err(|e| e.to_string())?.flatten().collect();

            // Drop stmt before using conn again
            drop(stmt);

            for (id, name, project_id, server_type, command, args, url, env, source_path, registry_plugin_id) in servers {
                let metadata = serde_json::json!({
                    "server_type": server_type,
                    "command": command,
                    "args": args,
                    "url": url,
                    "env": env,
                    "registry_plugin_id": registry_plugin_id,
                });

                let scope = if project_id.is_some() { "project" } else { "global" };

                // Idempotency: skip if already exists
                let exists: bool = conn.query_row(
                    "SELECT COUNT(*) > 0 FROM resources WHERE id = ?1",
                    [&id], |row| row.get(0)
                ).unwrap_or(false);

                if !exists {
                    let now = chrono::Utc::now().to_rfc3339();
                    conn.execute(
                        "INSERT INTO resources (id, resource_type, name, description, scope, source_path, content_hash, metadata, created_at, updated_at)
                         VALUES (?1, 'mcp_server', ?2, NULL, ?3, ?4, NULL, ?5, ?6, ?7)",
                        rusqlite::params![id, name, scope, source_path, metadata.to_string(), now, now],
                    ).map_err(|e| e.to_string())?;
                }
            }
        }

        // Set schema version
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('schema_version', '3')",
            [],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn migrate_v4_to_v5(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let current_version: Option<String> = conn
            .query_row("SELECT value FROM app_settings WHERE key = 'schema_version'", [], |row| row.get(0))
            .ok();
        let ver = current_version.as_deref().and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
        if ver >= 5 {
            return Ok(());
        }
        let has_version: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('resources') WHERE name='version'", [], |row| row.get(0)
        ).unwrap_or(0);
        if has_version == 0 {
            conn.execute("ALTER TABLE resources ADD COLUMN version TEXT", []).map_err(|e| e.to_string())?;
        }
        let has_draft: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('resources') WHERE name='is_draft'", [], |row| row.get(0)
        ).unwrap_or(0);
        if has_draft == 0 {
            conn.execute("ALTER TABLE resources ADD COLUMN is_draft INTEGER DEFAULT 1", []).map_err(|e| e.to_string())?;
        }
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS resource_versions (
                id              TEXT PRIMARY KEY,
                resource_id     TEXT NOT NULL,
                version         TEXT NOT NULL,
                changelog       TEXT,
                content_hash    TEXT NOT NULL,
                created_at      TEXT NOT NULL,
                FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE CASCADE,
                UNIQUE(resource_id, version)
            );"
        ).map_err(|e| e.to_string())?;
        conn.execute("INSERT OR REPLACE INTO app_settings (key, value) VALUES ('schema_version', '5')", []).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn migrate_v5_to_v6(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let current_version: Option<String> = conn
            .query_row("SELECT value FROM app_settings WHERE key = 'schema_version'", [], |row| row.get(0))
            .ok();
        let ver = current_version.as_deref().and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
        if ver >= 6 {
            return Ok(());
        }
        // Remove duplicate resources (keep the one with lowest rowid)
        conn.execute(
            "DELETE FROM resources WHERE rowid NOT IN (SELECT MIN(rowid) FROM resources GROUP BY name, scope, source_path)",
            [],
        ).map_err(|e| e.to_string())?;
        // Add unique index to prevent future duplicates
        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_resources_unique ON resources(name, scope, source_path)",
            [],
        ).map_err(|e| e.to_string())?;
        conn.execute("INSERT OR REPLACE INTO app_settings (key, value) VALUES ('schema_version', '6')", []).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn migrate_v6_to_v7(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("DROP TABLE IF EXISTS mcp_servers;")
            .map_err(|e| format!("migrate_v6_to_v7 failed: {}", e))?;
        Ok(())
    }

    pub fn migrate_v7_to_v8(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let has_column: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('resource_links') WHERE name='installed_hash'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        if has_column == 0 {
            conn.execute("ALTER TABLE resource_links ADD COLUMN installed_hash TEXT", [])
                .map_err(|e| format!("migrate_v7_to_v8 failed: {}", e))?;
        }
        Ok(())
    }

    pub fn migrate_v8_to_v9(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_links_resource_id ON resource_links(resource_id);
             CREATE INDEX IF NOT EXISTS idx_links_project_id ON resource_links(project_id);
             CREATE INDEX IF NOT EXISTS idx_resources_scope ON resources(scope);
             CREATE INDEX IF NOT EXISTS idx_resources_scope_type ON resources(scope, resource_type);"
        ).map_err(|e| format!("migrate_v8_to_v9 failed: {}", e))?;
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('schema_version', '9')",
            [],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn migrate_v9_to_v10(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let has_column: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('resources') WHERE name='installed_from_id'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        if has_column == 0 {
            conn.execute("ALTER TABLE resources ADD COLUMN installed_from_id TEXT", [])
                .map_err(|e| format!("add installed_from_id column failed: {}", e))?;
        }
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_resources_installed_from ON resources(installed_from_id)",
            [],
        ).map_err(|e| format!("create index failed: {}", e))?;

        // Clean up legacy symlink links — installed_from_id replaces them
        conn.execute("DELETE FROM resource_links WHERE link_type = 'symlink'", [])
            .map_err(|e| format!("migrate_v9_to_v10: symlink link cleanup failed: {}", e))?;

        // One-time: generate .ccm.json manifests for existing installed/ files
        {
            // Release the conn lock before calling self methods
            drop(conn);

            if let Some(home) = dirs::home_dir() {
                let installed_base = home.join(".claude-manager").join("installed");
                if installed_base.is_dir() {
                    let lib_resources = self.list_resources_by_scope(&crate::models::v2::ResourceScope::Library).unwrap_or_default();
                    let reg_resources = self.list_resources_by_scope(&crate::models::v2::ResourceScope::Registry).unwrap_or_default();

                    for type_dir in &["skills", "agents", "rules", "commands"] {
                        let dir = installed_base.join(type_dir);
                        if !dir.is_dir() { continue; }
                        if let Ok(entries) = std::fs::read_dir(&dir) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                // Skip manifest files themselves
                                if path.extension().and_then(|e| e.to_str()) == Some("json")
                                   && path.to_string_lossy().ends_with(".ccm.json") {
                                    continue;
                                }
                                let manifest_path = {
                                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                                    path.with_file_name(format!("{}.ccm.json", name))
                                };
                                if manifest_path.exists() {
                                    continue; // Already has manifest
                                }
                                // Match by name
                                let name = path.file_name().unwrap().to_string_lossy().to_string();
                                let name_stem = name.strip_suffix(".md").unwrap_or(&name);
                                let source = lib_resources.iter()
                                    .chain(reg_resources.iter())
                                    .find(|r| r.name == name_stem || r.name == name);
                                if let Some(src) = source {
                                    let manifest = serde_json::json!({
                                        "source_id": src.id,
                                        "source_scope": src.scope.as_str(),
                                        "source_name": src.name,
                                        "installed_at": src.updated_at,
                                    });
                                    let _ = std::fs::write(&manifest_path,
                                        serde_json::to_string_pretty(&manifest).unwrap_or_default());
                                }
                            }
                        }
                    }
                }
            }

            // Re-acquire conn for the final version update
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('schema_version', '10')",
                [],
            ).map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub fn migrate_v10_to_v11(&self) -> Result<(), String> {
        // For all resources with installed_from_id set, create a resource_links record
        // if one doesn't already exist. This makes resource_links the single source of truth
        // for install tracking (reversing the v9→v10 decision to delete symlink links).
        let conn = self.conn.lock().unwrap();

        // Check if migration already ran
        let version: String = conn.query_row(
            "SELECT COALESCE((SELECT value FROM app_settings WHERE key = 'schema_version'), '0')",
            [], |row| row.get(0),
        ).unwrap_or_else(|_| "0".to_string());
        if version.parse::<i32>().unwrap_or(0) >= 11 {
            return Ok(());
        }

        // Find all resources with installed_from_id that don't have a corresponding link
        let mut stmt = conn.prepare(
            "SELECT id, resource_type, name, scope, source_path, content_hash, installed_from_id
             FROM resources
             WHERE installed_from_id IS NOT NULL AND installed_from_id != ''"
        ).map_err(|e| format!("prepare failed: {}", e))?;

        let rows: Vec<(String, String, String, String, String, Option<String>, String)> = stmt.query_map([], |row| {
            Ok((
                row.get(0)?, row.get(1)?, row.get(2)?,
                row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?,
            ))
        }).map_err(|e| format!("query failed: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

        drop(stmt);

        let now = chrono::Utc::now().to_rfc3339();
        for (_id, _rtype, _name, scope, source_path, content_hash, installed_from_id) in &rows {
            // Check if source resource still exists
            let source_exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM resources WHERE id = ?1",
                rusqlite::params![installed_from_id],
                |row| row.get(0),
            ).unwrap_or(0);
            if source_exists == 0 {
                continue; // Skip orphaned references
            }

            // Check if a link already exists for this source → target
            let existing: i64 = conn.query_row(
                "SELECT COUNT(*) FROM resource_links WHERE resource_id = ?1 AND target_path = ?2",
                rusqlite::params![installed_from_id, source_path],
                |row| row.get(0),
            ).unwrap_or(0);

            if existing == 0 {
                // Determine project_id for project-scoped resources
                let project_id: Option<String> = if scope == "project" {
                    conn.query_row(
                        "SELECT id FROM projects WHERE ?1 LIKE path || '%'",
                        rusqlite::params![source_path],
                        |row| row.get(0),
                    ).ok()
                } else {
                    None
                };

                let link_id = uuid::Uuid::new_v4().to_string();
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO resource_links (id, resource_id, target_scope, target_path, config_key, project_id, link_type, created_at, installed_hash)
                     VALUES (?1, ?2, ?3, ?4, NULL, ?5, 'symlink', ?6, ?7)",
                    rusqlite::params![link_id, installed_from_id, scope, source_path, project_id, now, content_hash],
                );
            }
        }

        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('schema_version', '11')",
            [],
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn migrate_v3_to_v4(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let current_version: Option<String> = conn
            .query_row("SELECT value FROM app_settings WHERE key = 'schema_version'", [], |row| row.get(0))
            .ok();
        if current_version.as_deref() == Some("4") || current_version.as_deref().and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) >= 4 {
            return Ok(());
        }
        let has_pinned: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name='pinned'", [], |row| row.get(0)
        ).unwrap_or(0);
        if has_pinned == 0 {
            conn.execute("ALTER TABLE projects ADD COLUMN pinned INTEGER DEFAULT 0", []).map_err(|e| e.to_string())?;
        }
        let has_launch_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name='launch_count'", [], |row| row.get(0)
        ).unwrap_or(0);
        if has_launch_count == 0 {
            conn.execute("ALTER TABLE projects ADD COLUMN launch_count INTEGER DEFAULT 0", []).map_err(|e| e.to_string())?;
        }
        conn.execute("INSERT OR REPLACE INTO app_settings (key, value) VALUES ('schema_version', '4')", []).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_init_in_memory() {
        let db = Database::new_in_memory().unwrap();
        // Verify tables exist by querying them
        let conn = db.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM resources", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_get_setting_returns_none_when_missing() {
        let db = Database::new_in_memory().unwrap();
        let result = db.get_setting("nonexistent").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_set_and_get_setting() {
        let db = Database::new_in_memory().unwrap();
        db.set_setting("terminal_app", "iTerm2").unwrap();
        let result = db.get_setting("terminal_app").unwrap();
        assert_eq!(result, Some("iTerm2".to_string()));
    }

    #[test]
    fn test_set_setting_upsert() {
        let db = Database::new_in_memory().unwrap();
        db.set_setting("terminal_app", "Terminal").unwrap();
        db.set_setting("terminal_app", "Warp").unwrap();
        let result = db.get_setting("terminal_app").unwrap();
        assert_eq!(result, Some("Warp".to_string()));
    }

    #[test]
    fn test_migrate_v2_to_v3_adds_config_key() {
        let db = Database::new_in_memory().unwrap();
        // The in-memory DB already has config_key from init_tables
        // But migration should still work (idempotent)
        db.migrate_v2_to_v3().unwrap();
        let ver = db.get_setting("schema_version").unwrap();
        assert_eq!(ver, Some("3".to_string()));
    }

    #[test]
    fn test_migrate_v2_to_v3_idempotent() {
        let db = Database::new_in_memory().unwrap();
        db.migrate_v2_to_v3().unwrap();
        db.migrate_v2_to_v3().unwrap(); // Should not error
        let ver = db.get_setting("schema_version").unwrap();
        assert_eq!(ver, Some("3".to_string()));
    }

    #[test]
    fn test_migrate_v3_to_v4_adds_columns() {
        let db = Database::new_in_memory().unwrap();
        db.migrate_v3_to_v4().unwrap();
        let conn = db.conn.lock().unwrap();
        conn.execute("INSERT INTO projects (id, name, path, pinned, launch_count) VALUES ('t1', 'test', '/tmp/test', 1, 5)", []).unwrap();
        let pinned: i32 = conn.query_row("SELECT pinned FROM projects WHERE id = 't1'", [], |row| row.get(0)).unwrap();
        assert_eq!(pinned, 1);
        let count: i32 = conn.query_row("SELECT launch_count FROM projects WHERE id = 't1'", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 5);
    }

    #[test]
    fn test_migrate_v3_to_v4_idempotent() {
        let db = Database::new_in_memory().unwrap();
        db.migrate_v3_to_v4().unwrap();
        db.migrate_v3_to_v4().unwrap();
        let ver = db.get_setting("schema_version").unwrap();
        assert_eq!(ver, Some("4".to_string()));
    }

    #[test]
    fn test_migrate_v4_to_v5_adds_version_columns() {
        let db = Database::new_in_memory().unwrap();
        db.migrate_v4_to_v5().unwrap();
        {
            let conn = db.conn.lock().unwrap();
            let has_version: i64 = conn.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('resources') WHERE name='version'",
                [], |row| row.get(0)
            ).unwrap();
            assert_eq!(has_version, 1);
            let has_draft: i64 = conn.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('resources') WHERE name='is_draft'",
                [], |row| row.get(0)
            ).unwrap();
            assert_eq!(has_draft, 1);
            let _count: i64 = conn.query_row("SELECT COUNT(*) FROM resource_versions", [], |row| row.get(0)).unwrap();
        }
        let ver = db.get_setting("schema_version").unwrap();
        assert_eq!(ver, Some("5".to_string()));
    }

    #[test]
    fn test_migrate_v4_to_v5_idempotent() {
        let db = Database::new_in_memory().unwrap();
        db.migrate_v4_to_v5().unwrap();
        db.migrate_v4_to_v5().unwrap();
        let ver = db.get_setting("schema_version").unwrap();
        assert_eq!(ver, Some("5".to_string()));
    }

    #[test]
    fn test_migrate_v7_to_v8_adds_installed_hash() {
        let db = Database::new_in_memory().unwrap();
        db.migrate_v7_to_v8().unwrap();
        let conn = db.conn.lock().unwrap();
        let has_column: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('resource_links') WHERE name='installed_hash'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(has_column, 1);
    }

    #[test]
    fn test_migrate_v7_to_v8_is_idempotent() {
        let db = Database::new_in_memory().unwrap();
        db.migrate_v7_to_v8().unwrap();
        db.migrate_v7_to_v8().unwrap(); // Should not error
        let conn = db.conn.lock().unwrap();
        let has_column: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('resource_links') WHERE name='installed_hash'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(has_column, 1);
    }

    #[test]
    fn test_migrate_v8_to_v9_creates_indices() {
        let db = Database::new_in_memory().unwrap();
        db.migrate_v8_to_v9().unwrap();
        {
            let conn = db.conn.lock().unwrap();
            // Verify all four indices exist
            for index_name in &[
                "idx_links_resource_id",
                "idx_links_project_id",
                "idx_resources_scope",
                "idx_resources_scope_type",
            ] {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                    rusqlite::params![index_name],
                    |row| row.get(0),
                ).unwrap();
                assert_eq!(count, 1, "Index {} should exist", index_name);
            }
        }
        let ver = db.get_setting("schema_version").unwrap();
        assert_eq!(ver, Some("9".to_string()));
    }

    #[test]
    fn test_migrate_v8_to_v9_is_idempotent() {
        let db = Database::new_in_memory().unwrap();
        db.migrate_v8_to_v9().unwrap();
        db.migrate_v8_to_v9().unwrap(); // CREATE INDEX IF NOT EXISTS — should not error
        let ver = db.get_setting("schema_version").unwrap();
        assert_eq!(ver, Some("9".to_string()));
    }
}
