use rusqlite::{Connection, Result};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub mod resources;
pub mod projects;
pub mod plugins;
pub mod mcp;
pub mod links;
pub mod sync_state;
pub mod migration;
pub mod env;
pub mod registry;
pub mod library_plugin;

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
                updated_at      TEXT NOT NULL
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
            CREATE TABLE IF NOT EXISTS mcp_servers (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                project_id      TEXT,
                server_type     TEXT,
                command         TEXT,
                args            TEXT,
                url             TEXT,
                env             TEXT,
                source_path     TEXT NOT NULL,
                registry_plugin_id TEXT,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                FOREIGN KEY (registry_plugin_id) REFERENCES registry_plugins(id) ON DELETE CASCADE
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
        // Migration: add registry_plugin_id to mcp_servers if not exists
        let has_col: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('mcp_servers') WHERE name='registry_plugin_id'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        if has_col == 0 {
            conn.execute_batch(
                "ALTER TABLE mcp_servers ADD COLUMN registry_plugin_id TEXT REFERENCES registry_plugins(id) ON DELETE CASCADE;"
            )?;
        }

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

        if current_version.as_deref() == Some("3") {
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

        // Migrate mcp_servers rows to resources table
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

        // Set schema version
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('schema_version', '3')",
            [],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn migrate_v3_to_v4(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let current_version: Option<String> = conn
            .query_row("SELECT value FROM app_settings WHERE key = 'schema_version'", [], |row| row.get(0))
            .ok();
        if current_version.as_deref() == Some("4") {
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
    fn test_migrate_mcp_servers_to_resources() {
        use crate::models::v2::{McpServer, ResourceScope, ResourceType};

        let db = Database::new_in_memory().unwrap();

        // Insert project so FK constraint is satisfied
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO projects (id, name, path) VALUES ('p1', 'project-p1', '/tmp/project-p1')",
                [],
            ).unwrap();
            // Insert registry + registry_plugin so FK for registry_plugin_id is satisfied
            conn.execute(
                "INSERT INTO registries (id, name, url, local_path, readonly, has_remote_changes, has_local_changes, created_at)
                 VALUES ('reg1', 'registry-reg1', 'https://example.com', '/tmp/reg', 0, 0, 0, '2026-01-01')",
                [],
            ).unwrap();
            conn.execute(
                "INSERT INTO registry_plugins (id, registry_id, name, source_path, source_type)
                 VALUES ('rp1', 'reg1', 'plugin-rp1', '/tmp/plugin', 'local')",
                [],
            ).unwrap();
        }

        // Insert MCP server in old table
        db.insert_mcp_server(&McpServer {
            id: "m1".into(),
            name: "my-server".into(),
            project_id: Some("p1".into()),
            server_type: Some("stdio".into()),
            command: Some("node".into()),
            args: Some("[\"server.js\"]".into()),
            url: None,
            env: None,
            source_path: "/tmp/test/.mcp.json".into(),
            registry_plugin_id: Some("rp1".into()),
        }).unwrap();

        db.migrate_v2_to_v3().unwrap();

        // Verify resource was created
        let resources = db
            .list_resources_by_scope_and_type(&ResourceScope::Project, &ResourceType::McpServer)
            .unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "my-server");
        assert_eq!(resources[0].scope, ResourceScope::Project);

        // Verify metadata contains registry_plugin_id
        let metadata: serde_json::Value =
            serde_json::from_str(resources[0].metadata.as_deref().unwrap()).unwrap();
        assert_eq!(metadata["registry_plugin_id"], "rp1");
        assert_eq!(metadata["command"], "node");
        assert_eq!(metadata["server_type"], "stdio");

        // Verify idempotency: running migration again should not duplicate
        db.migrate_v2_to_v3().unwrap();
        let resources_again = db
            .list_resources_by_scope_and_type(&ResourceScope::Project, &ResourceType::McpServer)
            .unwrap();
        assert_eq!(resources_again.len(), 1);
    }

    #[test]
    fn test_migrate_global_mcp_server_to_resources() {
        use crate::models::v2::{McpServer, ResourceScope, ResourceType};

        let db = Database::new_in_memory().unwrap();

        // Insert a global MCP server (no project_id)
        db.insert_mcp_server(&McpServer {
            id: "m2".into(),
            name: "global-server".into(),
            project_id: None,
            server_type: Some("sse".into()),
            command: None,
            args: None,
            url: Some("http://localhost:3000".into()),
            env: None,
            source_path: "/tmp/global/.mcp.json".into(),
            registry_plugin_id: None,
        }).unwrap();

        db.migrate_v2_to_v3().unwrap();

        // Should be scope = "global"
        let resources = db
            .list_resources_by_scope_and_type(&ResourceScope::Global, &ResourceType::McpServer)
            .unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "global-server");
        assert_eq!(resources[0].scope, ResourceScope::Global);

        let metadata: serde_json::Value =
            serde_json::from_str(resources[0].metadata.as_deref().unwrap()).unwrap();
        assert_eq!(metadata["url"], "http://localhost:3000");
        assert_eq!(metadata["server_type"], "sse");
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
}
