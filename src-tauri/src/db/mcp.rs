use crate::db::Database;
use crate::models::v2::McpServer;
use rusqlite::{params, Result};

impl Database {
    pub fn insert_mcp_server(&self, server: &McpServer) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO mcp_servers (id, name, project_id, server_type, command, args, url, env, source_path, registry_plugin_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                server.id,
                server.name,
                server.project_id,
                server.server_type,
                server.command,
                server.args,
                server.url,
                server.env,
                server.source_path,
                server.registry_plugin_id,
            ],
        )?;
        Ok(())
    }

    pub fn get_mcp_server(&self, id: &str) -> Result<Option<McpServer>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, project_id, server_type, command, args, url, env, source_path, registry_plugin_id
             FROM mcp_servers WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(McpServer {
                id: row.get(0)?,
                name: row.get(1)?,
                project_id: row.get(2)?,
                server_type: row.get(3)?,
                command: row.get(4)?,
                args: row.get(5)?,
                url: row.get(6)?,
                env: row.get(7)?,
                source_path: row.get(8)?,
                registry_plugin_id: row.get(9)?,
            })
        })?;
        match rows.next() {
            Some(result) => Ok(Some(result?)),
            None => Ok(None),
        }
    }

    pub fn list_mcp_servers(&self) -> Result<Vec<McpServer>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, project_id, server_type, command, args, url, env, source_path, registry_plugin_id
             FROM mcp_servers",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(McpServer {
                id: row.get(0)?,
                name: row.get(1)?,
                project_id: row.get(2)?,
                server_type: row.get(3)?,
                command: row.get(4)?,
                args: row.get(5)?,
                url: row.get(6)?,
                env: row.get(7)?,
                source_path: row.get(8)?,
                registry_plugin_id: row.get(9)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_mcp_servers_by_project(&self, project_id: &str) -> Result<Vec<McpServer>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, project_id, server_type, command, args, url, env, source_path, registry_plugin_id
             FROM mcp_servers WHERE project_id = ?1",
        )?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok(McpServer {
                id: row.get(0)?,
                name: row.get(1)?,
                project_id: row.get(2)?,
                server_type: row.get(3)?,
                command: row.get(4)?,
                args: row.get(5)?,
                url: row.get(6)?,
                env: row.get(7)?,
                source_path: row.get(8)?,
                registry_plugin_id: row.get(9)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_global_mcp_servers(&self) -> Result<Vec<McpServer>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, project_id, server_type, command, args, url, env, source_path, registry_plugin_id
             FROM mcp_servers WHERE project_id IS NULL AND registry_plugin_id IS NULL",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(McpServer {
                id: row.get(0)?,
                name: row.get(1)?,
                project_id: row.get(2)?,
                server_type: row.get(3)?,
                command: row.get(4)?,
                args: row.get(5)?,
                url: row.get(6)?,
                env: row.get(7)?,
                source_path: row.get(8)?,
                registry_plugin_id: row.get(9)?,
            })
        })?;
        rows.collect()
    }

    pub fn update_mcp_server(&self, server: &McpServer) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE mcp_servers SET name = ?1, project_id = ?2, server_type = ?3, command = ?4,
             args = ?5, url = ?6, env = ?7, source_path = ?8, registry_plugin_id = ?9 WHERE id = ?10",
            params![
                server.name,
                server.project_id,
                server.server_type,
                server.command,
                server.args,
                server.url,
                server.env,
                server.source_path,
                server.registry_plugin_id,
                server.id,
            ],
        )?;
        Ok(())
    }

    pub fn delete_mcp_server(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM mcp_servers WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn delete_mcp_servers_by_source(&self, source_path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM mcp_servers WHERE source_path = ?1",
            params![source_path],
        )?;
        Ok(())
    }

    pub fn list_mcp_servers_by_registry_plugin(&self, registry_plugin_id: &str) -> Result<Vec<McpServer>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, project_id, server_type, command, args, url, env, source_path, registry_plugin_id
             FROM mcp_servers WHERE registry_plugin_id = ?1",
        )?;
        let rows = stmt.query_map(params![registry_plugin_id], |row| {
            Ok(McpServer {
                id: row.get(0)?,
                name: row.get(1)?,
                project_id: row.get(2)?,
                server_type: row.get(3)?,
                command: row.get(4)?,
                args: row.get(5)?,
                url: row.get(6)?,
                env: row.get(7)?,
                source_path: row.get(8)?,
                registry_plugin_id: row.get(9)?,
            })
        })?;
        rows.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mcp_server(id: &str, project_id: Option<String>) -> McpServer {
        McpServer {
            id: id.to_string(),
            name: format!("server-{}", id),
            project_id,
            server_type: Some("stdio".to_string()),
            command: Some("node".to_string()),
            args: Some(r#"["server.js"]"#.to_string()),
            url: None,
            env: None,
            source_path: "/tmp/.mcp.json".to_string(),
            registry_plugin_id: None,
        }
    }

    /// Helper: insert a registry + registry_plugin so FK constraints are satisfied.
    fn insert_test_registry_plugin(db: &Database, plugin_id: &str, registry_id: &str) {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO registries (id, name, url, local_path, readonly, has_remote_changes, has_local_changes, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, 0, 0, '2026-01-01')",
            params![registry_id, format!("registry-{}", registry_id), "https://example.com", "/tmp/reg"],
        ).unwrap();
        conn.execute(
            "INSERT INTO registry_plugins (id, registry_id, name, source_path, source_type)
             VALUES (?1, ?2, ?3, ?4, 'local')",
            params![plugin_id, registry_id, format!("plugin-{}", plugin_id), "/tmp/plugin"],
        ).unwrap();
    }

    /// Helper: insert a project row directly so FK constraints are satisfied.
    fn insert_test_project(db: &Database, id: &str) {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, path) VALUES (?1, ?2, ?3)",
            params![id, format!("project-{}", id), format!("/tmp/project-{}", id)],
        )
        .unwrap();
    }

    #[test]
    fn test_insert_and_get_global_server() {
        let db = Database::new_in_memory().unwrap();
        let server = make_mcp_server("s1", None);
        db.insert_mcp_server(&server).unwrap();

        let fetched = db.get_mcp_server("s1").unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, "s1");
        assert_eq!(fetched.name, "server-s1");
        assert_eq!(fetched.project_id, None);
        assert_eq!(fetched.server_type, Some("stdio".to_string()));
        assert_eq!(fetched.command, Some("node".to_string()));
        assert_eq!(fetched.args, Some(r#"["server.js"]"#.to_string()));
        assert_eq!(fetched.url, None);
        assert_eq!(fetched.env, None);
        assert_eq!(fetched.source_path, "/tmp/.mcp.json");
    }

    #[test]
    fn test_insert_and_get_project_server() {
        let db = Database::new_in_memory().unwrap();
        insert_test_project(&db, "proj1");

        let server = make_mcp_server("s2", Some("proj1".to_string()));
        db.insert_mcp_server(&server).unwrap();

        let fetched = db.get_mcp_server("s2").unwrap().unwrap();
        assert_eq!(fetched.project_id, Some("proj1".to_string()));
    }

    #[test]
    fn test_get_nonexistent_server() {
        let db = Database::new_in_memory().unwrap();
        let result = db.get_mcp_server("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_mcp_servers() {
        let db = Database::new_in_memory().unwrap();
        insert_test_project(&db, "proj1");

        let s1 = make_mcp_server("s1", None);
        let s2 = make_mcp_server("s2", Some("proj1".to_string()));
        let s3 = make_mcp_server("s3", None);
        db.insert_mcp_server(&s1).unwrap();
        db.insert_mcp_server(&s2).unwrap();
        db.insert_mcp_server(&s3).unwrap();

        let all = db.list_mcp_servers().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_list_mcp_servers_by_project() {
        let db = Database::new_in_memory().unwrap();
        insert_test_project(&db, "proj1");
        insert_test_project(&db, "proj2");

        let s1 = make_mcp_server("s1", Some("proj1".to_string()));
        let s2 = make_mcp_server("s2", Some("proj1".to_string()));
        let s3 = make_mcp_server("s3", Some("proj2".to_string()));
        let s4 = make_mcp_server("s4", None);
        db.insert_mcp_server(&s1).unwrap();
        db.insert_mcp_server(&s2).unwrap();
        db.insert_mcp_server(&s3).unwrap();
        db.insert_mcp_server(&s4).unwrap();

        let proj1_servers = db.list_mcp_servers_by_project("proj1").unwrap();
        assert_eq!(proj1_servers.len(), 2);
        assert!(proj1_servers.iter().all(|s| s.project_id == Some("proj1".to_string())));

        let proj2_servers = db.list_mcp_servers_by_project("proj2").unwrap();
        assert_eq!(proj2_servers.len(), 1);
        assert_eq!(proj2_servers[0].id, "s3");
    }

    #[test]
    fn test_list_global_mcp_servers() {
        let db = Database::new_in_memory().unwrap();
        insert_test_project(&db, "proj1");

        let s1 = make_mcp_server("s1", None);
        let s2 = make_mcp_server("s2", Some("proj1".to_string()));
        let s3 = make_mcp_server("s3", None);
        db.insert_mcp_server(&s1).unwrap();
        db.insert_mcp_server(&s2).unwrap();
        db.insert_mcp_server(&s3).unwrap();

        let global = db.list_global_mcp_servers().unwrap();
        assert_eq!(global.len(), 2);
        assert!(global.iter().all(|s| s.project_id.is_none()));
    }

    #[test]
    fn test_update_mcp_server() {
        let db = Database::new_in_memory().unwrap();
        let server = make_mcp_server("s1", None);
        db.insert_mcp_server(&server).unwrap();

        let mut updated = server.clone();
        updated.name = "updated-server".to_string();
        updated.server_type = Some("sse".to_string());
        updated.command = None;
        updated.args = None;
        updated.url = Some("http://localhost:3000".to_string());
        updated.env = Some(r#"{"API_KEY":"secret"}"#.to_string());
        updated.source_path = "/tmp/updated.json".to_string();
        db.update_mcp_server(&updated).unwrap();

        let fetched = db.get_mcp_server("s1").unwrap().unwrap();
        assert_eq!(fetched.name, "updated-server");
        assert_eq!(fetched.server_type, Some("sse".to_string()));
        assert_eq!(fetched.command, None);
        assert_eq!(fetched.args, None);
        assert_eq!(fetched.url, Some("http://localhost:3000".to_string()));
        assert_eq!(fetched.env, Some(r#"{"API_KEY":"secret"}"#.to_string()));
        assert_eq!(fetched.source_path, "/tmp/updated.json");
    }

    #[test]
    fn test_update_mcp_server_project_id() {
        let db = Database::new_in_memory().unwrap();
        insert_test_project(&db, "proj1");

        // Start as global, then assign to project
        let server = make_mcp_server("s1", None);
        db.insert_mcp_server(&server).unwrap();

        let mut updated = server.clone();
        updated.project_id = Some("proj1".to_string());
        db.update_mcp_server(&updated).unwrap();

        let fetched = db.get_mcp_server("s1").unwrap().unwrap();
        assert_eq!(fetched.project_id, Some("proj1".to_string()));
    }

    #[test]
    fn test_delete_mcp_server() {
        let db = Database::new_in_memory().unwrap();
        let server = make_mcp_server("s1", None);
        db.insert_mcp_server(&server).unwrap();

        db.delete_mcp_server("s1").unwrap();
        let fetched = db.get_mcp_server("s1").unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn test_delete_nonexistent_server() {
        let db = Database::new_in_memory().unwrap();
        // Should not error even if no rows are deleted
        db.delete_mcp_server("nonexistent").unwrap();
    }

    #[test]
    fn test_delete_mcp_servers_by_source() {
        let db = Database::new_in_memory().unwrap();
        let mut s1 = make_mcp_server("s1", None);
        s1.source_path = "/home/user/.claude.json".to_string();
        let mut s2 = make_mcp_server("s2", None);
        s2.source_path = "/home/user/.claude.json".to_string();
        let mut s3 = make_mcp_server("s3", None);
        s3.source_path = "/other/path/.mcp.json".to_string();

        db.insert_mcp_server(&s1).unwrap();
        db.insert_mcp_server(&s2).unwrap();
        db.insert_mcp_server(&s3).unwrap();

        db.delete_mcp_servers_by_source("/home/user/.claude.json").unwrap();

        let all = db.list_mcp_servers().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "s3");
    }

    #[test]
    fn test_delete_mcp_servers_by_source_no_match() {
        let db = Database::new_in_memory().unwrap();
        let server = make_mcp_server("s1", None);
        db.insert_mcp_server(&server).unwrap();

        // Deleting by a source path that doesn't match should leave data intact
        db.delete_mcp_servers_by_source("/nonexistent/path").unwrap();
        let all = db.list_mcp_servers().unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_insert_duplicate_id_fails() {
        let db = Database::new_in_memory().unwrap();
        let s1 = make_mcp_server("s1", None);
        db.insert_mcp_server(&s1).unwrap();

        let s1_dup = make_mcp_server("s1", None);
        let result = db.insert_mcp_server(&s1_dup);
        assert!(result.is_err());
    }

    #[test]
    fn test_insert_with_invalid_project_id_fk_fails() {
        let db = Database::new_in_memory().unwrap();
        // project_id references a non-existent project -- FK constraint should fail
        let server = make_mcp_server("s1", Some("nonexistent_project".to_string()));
        let result = db.insert_mcp_server(&server);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_mcp_servers_by_registry_plugin() {
        let db = Database::new_in_memory().unwrap();
        insert_test_registry_plugin(&db, "rp1", "reg1");
        insert_test_registry_plugin(&db, "rp2", "reg1");

        let mut s1 = make_mcp_server("s1", None);
        s1.registry_plugin_id = Some("rp1".to_string());
        let mut s2 = make_mcp_server("s2", None);
        s2.registry_plugin_id = Some("rp1".to_string());
        let mut s3 = make_mcp_server("s3", None);
        s3.registry_plugin_id = Some("rp2".to_string());
        let s4 = make_mcp_server("s4", None); // global, no registry_plugin_id

        db.insert_mcp_server(&s1).unwrap();
        db.insert_mcp_server(&s2).unwrap();
        db.insert_mcp_server(&s3).unwrap();
        db.insert_mcp_server(&s4).unwrap();

        let rp1_servers = db.list_mcp_servers_by_registry_plugin("rp1").unwrap();
        assert_eq!(rp1_servers.len(), 2);
        assert!(rp1_servers.iter().all(|s| s.registry_plugin_id == Some("rp1".to_string())));

        let rp2_servers = db.list_mcp_servers_by_registry_plugin("rp2").unwrap();
        assert_eq!(rp2_servers.len(), 1);
        assert_eq!(rp2_servers[0].id, "s3");
    }

    #[test]
    fn test_global_mcp_excludes_registry_plugin_servers() {
        let db = Database::new_in_memory().unwrap();
        insert_test_registry_plugin(&db, "rp1", "reg1");

        let s1 = make_mcp_server("s1", None); // true global
        let mut s2 = make_mcp_server("s2", None);
        s2.registry_plugin_id = Some("rp1".to_string()); // registry plugin, NOT global

        db.insert_mcp_server(&s1).unwrap();
        db.insert_mcp_server(&s2).unwrap();

        let global = db.list_global_mcp_servers().unwrap();
        assert_eq!(global.len(), 1);
        assert_eq!(global[0].id, "s1");
    }
}
