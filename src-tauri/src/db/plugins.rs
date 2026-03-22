use crate::db::Database;
use crate::models::v2::Plugin;
use rusqlite::{params, Result};

impl Database {
    pub fn insert_plugin(&self, plugin: &Plugin) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO plugins (id, name, version, scope, install_path, status, last_checked)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                plugin.id,
                plugin.name,
                plugin.version,
                plugin.scope,
                plugin.install_path,
                plugin.status,
                plugin.last_checked,
            ],
        )?;
        Ok(())
    }

    pub fn get_plugin(&self, id: &str) -> Result<Option<Plugin>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, version, scope, install_path, status, last_checked
             FROM plugins WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Plugin {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                scope: row.get(3)?,
                install_path: row.get(4)?,
                status: row.get(5)?,
                last_checked: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn list_plugins(&self) -> Result<Vec<Plugin>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, version, scope, install_path, status, last_checked
             FROM plugins ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Plugin {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                scope: row.get(3)?,
                install_path: row.get(4)?,
                status: row.get(5)?,
                last_checked: row.get(6)?,
            })
        })?;
        let mut plugins = Vec::new();
        for row in rows {
            plugins.push(row?);
        }
        Ok(plugins)
    }

    pub fn update_plugin(&self, plugin: &Plugin) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE plugins SET name = ?1, version = ?2, scope = ?3, install_path = ?4,
             status = ?5, last_checked = ?6 WHERE id = ?7",
            params![
                plugin.name,
                plugin.version,
                plugin.scope,
                plugin.install_path,
                plugin.status,
                plugin.last_checked,
                plugin.id,
            ],
        )?;
        Ok(())
    }

    pub fn delete_plugin(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM plugins WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn count_plugins(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM plugins", [], |row| row.get(0))
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::models::v2::Plugin;

    fn make_plugin(id: &str) -> Plugin {
        Plugin {
            id: id.to_string(),
            name: format!("plugin-{}", id),
            version: Some("1.0.0".to_string()),
            scope: Some("@claude".to_string()),
            install_path: Some(format!("/home/user/.claude/plugins/{}", id)),
            status: "installed".to_string(),
            last_checked: Some("2026-03-01T00:00:00Z".to_string()),
        }
    }

    #[test]
    fn test_insert_and_get_plugin() {
        let db = Database::new_in_memory().unwrap();
        let plugin = make_plugin("p1");
        db.insert_plugin(&plugin).unwrap();

        let fetched = db.get_plugin("p1").unwrap().unwrap();
        assert_eq!(fetched.id, "p1");
        assert_eq!(fetched.name, "plugin-p1");
        assert_eq!(fetched.version, Some("1.0.0".to_string()));
        assert_eq!(fetched.scope, Some("@claude".to_string()));
        assert_eq!(fetched.install_path, Some("/home/user/.claude/plugins/p1".to_string()));
        assert_eq!(fetched.status, "installed");
        assert_eq!(fetched.last_checked, Some("2026-03-01T00:00:00Z".to_string()));
    }

    #[test]
    fn test_get_plugin_not_found() {
        let db = Database::new_in_memory().unwrap();
        let result = db.get_plugin("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_insert_plugin_duplicate_id() {
        let db = Database::new_in_memory().unwrap();
        let plugin = make_plugin("dup");
        db.insert_plugin(&plugin).unwrap();
        let result = db.insert_plugin(&plugin);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_plugins_empty() {
        let db = Database::new_in_memory().unwrap();
        let plugins = db.list_plugins().unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_list_plugins_ordered_by_name() {
        let db = Database::new_in_memory().unwrap();
        // Insert in reverse order to verify ordering
        let mut p3 = make_plugin("3");
        p3.name = "zeta".to_string();
        let mut p1 = make_plugin("1");
        p1.name = "alpha".to_string();
        let mut p2 = make_plugin("2");
        p2.name = "mu".to_string();

        db.insert_plugin(&p3).unwrap();
        db.insert_plugin(&p1).unwrap();
        db.insert_plugin(&p2).unwrap();

        let plugins = db.list_plugins().unwrap();
        assert_eq!(plugins.len(), 3);
        assert_eq!(plugins[0].name, "alpha");
        assert_eq!(plugins[1].name, "mu");
        assert_eq!(plugins[2].name, "zeta");
    }

    #[test]
    fn test_update_plugin() {
        let db = Database::new_in_memory().unwrap();
        let plugin = make_plugin("u1");
        db.insert_plugin(&plugin).unwrap();

        let mut updated = plugin.clone();
        updated.name = "updated-name".to_string();
        updated.version = Some("2.0.0".to_string());
        updated.scope = None;
        updated.status = "disabled".to_string();
        updated.last_checked = None;

        db.update_plugin(&updated).unwrap();

        let fetched = db.get_plugin("u1").unwrap().unwrap();
        assert_eq!(fetched.name, "updated-name");
        assert_eq!(fetched.version, Some("2.0.0".to_string()));
        assert_eq!(fetched.scope, None);
        assert_eq!(fetched.status, "disabled");
        assert_eq!(fetched.last_checked, None);
    }

    #[test]
    fn test_update_nonexistent_plugin() {
        let db = Database::new_in_memory().unwrap();
        let plugin = make_plugin("ghost");
        // update on a nonexistent row succeeds but changes nothing
        db.update_plugin(&plugin).unwrap();
        let fetched = db.get_plugin("ghost").unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn test_delete_plugin() {
        let db = Database::new_in_memory().unwrap();
        let plugin = make_plugin("d1");
        db.insert_plugin(&plugin).unwrap();
        assert_eq!(db.count_plugins().unwrap(), 1);

        db.delete_plugin("d1").unwrap();
        assert_eq!(db.count_plugins().unwrap(), 0);
        assert!(db.get_plugin("d1").unwrap().is_none());
    }

    #[test]
    fn test_delete_nonexistent_plugin() {
        let db = Database::new_in_memory().unwrap();
        // Deleting a nonexistent row should not error
        db.delete_plugin("nope").unwrap();
    }

    #[test]
    fn test_count_plugins() {
        let db = Database::new_in_memory().unwrap();
        assert_eq!(db.count_plugins().unwrap(), 0);

        db.insert_plugin(&make_plugin("c1")).unwrap();
        assert_eq!(db.count_plugins().unwrap(), 1);

        db.insert_plugin(&make_plugin("c2")).unwrap();
        assert_eq!(db.count_plugins().unwrap(), 2);

        db.delete_plugin("c1").unwrap();
        assert_eq!(db.count_plugins().unwrap(), 1);
    }

    #[test]
    fn test_plugin_with_all_optional_fields_none() {
        let db = Database::new_in_memory().unwrap();
        let plugin = Plugin {
            id: "minimal".to_string(),
            name: "bare-plugin".to_string(),
            version: None,
            scope: None,
            install_path: None,
            status: "unknown".to_string(),
            last_checked: None,
        };
        db.insert_plugin(&plugin).unwrap();

        let fetched = db.get_plugin("minimal").unwrap().unwrap();
        assert_eq!(fetched.id, "minimal");
        assert_eq!(fetched.name, "bare-plugin");
        assert_eq!(fetched.version, None);
        assert_eq!(fetched.scope, None);
        assert_eq!(fetched.install_path, None);
        assert_eq!(fetched.status, "unknown");
        assert_eq!(fetched.last_checked, None);
    }
}
