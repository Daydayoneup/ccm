use crate::db::Database;
use crate::models::v2::Registry;
use rusqlite::{params, Result};

impl Database {
    pub fn insert_registry(&self, registry: &Registry) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO registries (id, name, url, local_path, readonly, last_synced, has_remote_changes, has_local_changes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                registry.id,
                registry.name,
                registry.url,
                registry.local_path,
                registry.readonly as i32,
                registry.last_synced,
                registry.has_remote_changes as i32,
                registry.has_local_changes as i32,
                registry.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_registry(&self, id: &str) -> Result<Option<Registry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, url, local_path, readonly, last_synced, has_remote_changes, has_local_changes, created_at
             FROM registries WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Registry {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                local_path: row.get(3)?,
                readonly: row.get::<_, i32>(4)? != 0,
                last_synced: row.get(5)?,
                has_remote_changes: row.get::<_, i32>(6)? != 0,
                has_local_changes: row.get::<_, i32>(7)? != 0,
                created_at: row.get(8)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn list_registries(&self) -> Result<Vec<Registry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, url, local_path, readonly, last_synced, has_remote_changes, has_local_changes, created_at
             FROM registries ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Registry {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                local_path: row.get(3)?,
                readonly: row.get::<_, i32>(4)? != 0,
                last_synced: row.get(5)?,
                has_remote_changes: row.get::<_, i32>(6)? != 0,
                has_local_changes: row.get::<_, i32>(7)? != 0,
                created_at: row.get(8)?,
            })
        })?;
        let mut registries = Vec::new();
        for row in rows {
            registries.push(row?);
        }
        Ok(registries)
    }

    pub fn update_registry(&self, registry: &Registry) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE registries SET name = ?1, url = ?2, local_path = ?3, readonly = ?4,
             last_synced = ?5, has_remote_changes = ?6, has_local_changes = ?7 WHERE id = ?8",
            params![
                registry.name,
                registry.url,
                registry.local_path,
                registry.readonly as i32,
                registry.last_synced,
                registry.has_remote_changes as i32,
                registry.has_local_changes as i32,
                registry.id,
            ],
        )?;
        Ok(())
    }

    pub fn delete_registry(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM registries WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_registry_by_url(&self, url: &str) -> Result<Option<Registry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, url, local_path, readonly, last_synced, has_remote_changes, has_local_changes, created_at
             FROM registries WHERE url = ?1",
        )?;
        let mut rows = stmt.query_map(params![url], |row| {
            Ok(Registry {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                local_path: row.get(3)?,
                readonly: row.get::<_, i32>(4)? != 0,
                last_synced: row.get(5)?,
                has_remote_changes: row.get::<_, i32>(6)? != 0,
                has_local_changes: row.get::<_, i32>(7)? != 0,
                created_at: row.get(8)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn count_registries(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM registries", [], |row| row.get(0))
    }
}

use crate::models::v2::RegistryPlugin;

impl Database {
    pub fn insert_registry_plugin(&self, plugin: &RegistryPlugin) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO registry_plugins (id, registry_id, name, description, category, source_path, source_type, source_url, homepage)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                plugin.id,
                plugin.registry_id,
                plugin.name,
                plugin.description,
                plugin.category,
                plugin.source_path,
                plugin.source_type,
                plugin.source_url,
                plugin.homepage,
            ],
        )?;
        Ok(())
    }

    pub fn get_registry_plugin(&self, id: &str) -> Result<Option<RegistryPlugin>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, registry_id, name, description, category, source_path, source_type, source_url, homepage
             FROM registry_plugins WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(RegistryPlugin {
                id: row.get(0)?,
                registry_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                category: row.get(4)?,
                source_path: row.get(5)?,
                source_type: row.get(6)?,
                source_url: row.get(7)?,
                homepage: row.get(8)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn list_registry_plugins(&self, registry_id: &str) -> Result<Vec<RegistryPlugin>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, registry_id, name, description, category, source_path, source_type, source_url, homepage
             FROM registry_plugins WHERE registry_id = ?1 ORDER BY name"
        )?;
        let rows = stmt.query_map(params![registry_id], |row| {
            Ok(RegistryPlugin {
                id: row.get(0)?,
                registry_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                category: row.get(4)?,
                source_path: row.get(5)?,
                source_type: row.get(6)?,
                source_url: row.get(7)?,
                homepage: row.get(8)?,
            })
        })?;
        let mut plugins = Vec::new();
        for row in rows {
            plugins.push(row?);
        }
        Ok(plugins)
    }

    pub fn delete_registry_plugins_by_registry(&self, registry_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM registry_plugins WHERE registry_id = ?1",
            params![registry_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::models::v2::Registry;
    use crate::models::v2::RegistryPlugin;

    fn make_registry(id: &str, name: &str, url: &str) -> Registry {
        Registry {
            id: id.to_string(),
            name: name.to_string(),
            url: url.to_string(),
            local_path: format!("/home/user/.claude-manager/registries/{}", id),
            readonly: true,
            last_synced: Some("2026-03-01T00:00:00Z".to_string()),
            has_remote_changes: false,
            has_local_changes: false,
            created_at: "2026-03-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_insert_and_get_registry() {
        let db = Database::new_in_memory().unwrap();
        let registry = make_registry("r1", "my-registry", "https://github.com/user/repo.git");
        db.insert_registry(&registry).unwrap();

        let fetched = db.get_registry("r1").unwrap().unwrap();
        assert_eq!(fetched.id, "r1");
        assert_eq!(fetched.name, "my-registry");
        assert_eq!(fetched.url, "https://github.com/user/repo.git");
        assert_eq!(fetched.local_path, "/home/user/.claude-manager/registries/r1");
        assert_eq!(fetched.readonly, true);
        assert_eq!(fetched.last_synced, Some("2026-03-01T00:00:00Z".to_string()));
        assert_eq!(fetched.has_remote_changes, false);
        assert_eq!(fetched.has_local_changes, false);
        assert_eq!(fetched.created_at, "2026-03-01T00:00:00Z");
    }

    #[test]
    fn test_get_registry_not_found() {
        let db = Database::new_in_memory().unwrap();
        let result = db.get_registry("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_duplicate_url_fails() {
        let db = Database::new_in_memory().unwrap();
        let r1 = make_registry("r1", "first", "https://github.com/user/repo.git");
        let r2 = make_registry("r2", "second", "https://github.com/user/repo.git");
        db.insert_registry(&r1).unwrap();
        let result = db.insert_registry(&r2);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_registries_ordered_by_name() {
        let db = Database::new_in_memory().unwrap();
        let r3 = make_registry("r3", "zeta-registry", "https://github.com/user/zeta.git");
        let r1 = make_registry("r1", "alpha-registry", "https://github.com/user/alpha.git");
        let r2 = make_registry("r2", "mu-registry", "https://github.com/user/mu.git");

        db.insert_registry(&r3).unwrap();
        db.insert_registry(&r1).unwrap();
        db.insert_registry(&r2).unwrap();

        let registries = db.list_registries().unwrap();
        assert_eq!(registries.len(), 3);
        assert_eq!(registries[0].name, "alpha-registry");
        assert_eq!(registries[1].name, "mu-registry");
        assert_eq!(registries[2].name, "zeta-registry");
    }

    #[test]
    fn test_update_registry() {
        let db = Database::new_in_memory().unwrap();
        let registry = make_registry("u1", "original", "https://github.com/user/repo.git");
        db.insert_registry(&registry).unwrap();

        let mut updated = registry.clone();
        updated.name = "updated-name".to_string();
        updated.url = "https://github.com/user/new-repo.git".to_string();
        updated.readonly = false;
        updated.last_synced = None;
        updated.has_remote_changes = true;
        updated.has_local_changes = true;

        db.update_registry(&updated).unwrap();

        let fetched = db.get_registry("u1").unwrap().unwrap();
        assert_eq!(fetched.name, "updated-name");
        assert_eq!(fetched.url, "https://github.com/user/new-repo.git");
        assert_eq!(fetched.readonly, false);
        assert_eq!(fetched.last_synced, None);
        assert_eq!(fetched.has_remote_changes, true);
        assert_eq!(fetched.has_local_changes, true);
    }

    #[test]
    fn test_delete_registry() {
        let db = Database::new_in_memory().unwrap();
        let registry = make_registry("d1", "to-delete", "https://github.com/user/repo.git");
        db.insert_registry(&registry).unwrap();
        assert_eq!(db.count_registries().unwrap(), 1);

        db.delete_registry("d1").unwrap();
        assert_eq!(db.count_registries().unwrap(), 0);
        assert!(db.get_registry("d1").unwrap().is_none());
    }

    #[test]
    fn test_get_registry_by_url() {
        let db = Database::new_in_memory().unwrap();
        let registry = make_registry("r1", "my-registry", "https://github.com/user/repo.git");
        db.insert_registry(&registry).unwrap();

        let fetched = db.get_registry_by_url("https://github.com/user/repo.git").unwrap().unwrap();
        assert_eq!(fetched.id, "r1");
        assert_eq!(fetched.name, "my-registry");

        let not_found = db.get_registry_by_url("https://github.com/other/repo.git").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_insert_and_get_registry_plugin() {
        let db = Database::new_in_memory().unwrap();
        let registry = make_registry("test-reg", "my-registry", "https://github.com/test/repo.git");
        db.insert_registry(&registry).unwrap();

        let plugin = RegistryPlugin {
            id: "rp-1".to_string(),
            registry_id: registry.id.clone(),
            name: "superpowers".to_string(),
            description: Some("Dev tools".to_string()),
            category: Some("development".to_string()),
            source_path: "/tmp/registries/repo/plugins/superpowers".to_string(),
            source_type: "local".to_string(),
            source_url: None,
            homepage: Some("https://github.com/obra/superpowers".to_string()),
        };
        db.insert_registry_plugin(&plugin).unwrap();

        let fetched = db.get_registry_plugin("rp-1").unwrap().unwrap();
        assert_eq!(fetched.name, "superpowers");
        assert_eq!(fetched.registry_id, registry.id);
        assert_eq!(fetched.source_type, "local");
    }

    #[test]
    fn test_list_registry_plugins_by_registry() {
        let db = Database::new_in_memory().unwrap();
        let registry = make_registry("test-reg", "my-registry", "https://github.com/test/repo.git");
        db.insert_registry(&registry).unwrap();

        let p1 = RegistryPlugin {
            id: "rp-1".to_string(),
            registry_id: registry.id.clone(),
            name: "alpha".to_string(),
            description: None,
            category: Some("development".to_string()),
            source_path: "/tmp/a".to_string(),
            source_type: "local".to_string(),
            source_url: None,
            homepage: None,
        };
        let p2 = RegistryPlugin {
            id: "rp-2".to_string(),
            registry_id: registry.id.clone(),
            name: "beta".to_string(),
            description: None,
            category: Some("productivity".to_string()),
            source_path: "/tmp/b".to_string(),
            source_type: "external".to_string(),
            source_url: Some("https://github.com/ext/beta.git".to_string()),
            homepage: None,
        };
        db.insert_registry_plugin(&p1).unwrap();
        db.insert_registry_plugin(&p2).unwrap();

        let plugins = db.list_registry_plugins(&registry.id).unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].name, "alpha");
        assert_eq!(plugins[1].name, "beta");
    }

    #[test]
    fn test_delete_registry_plugins_by_registry() {
        let db = Database::new_in_memory().unwrap();
        let registry = make_registry("test-reg", "my-registry", "https://github.com/test/repo.git");
        db.insert_registry(&registry).unwrap();

        let plugin = RegistryPlugin {
            id: "rp-1".to_string(),
            registry_id: registry.id.clone(),
            name: "test".to_string(),
            description: None,
            category: None,
            source_path: "/tmp/t".to_string(),
            source_type: "local".to_string(),
            source_url: None,
            homepage: None,
        };
        db.insert_registry_plugin(&plugin).unwrap();
        assert_eq!(db.list_registry_plugins(&registry.id).unwrap().len(), 1);

        db.delete_registry_plugins_by_registry(&registry.id).unwrap();
        assert_eq!(db.list_registry_plugins(&registry.id).unwrap().len(), 0);
    }
}
