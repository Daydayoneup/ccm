use crate::db::Database;
use crate::models::v2::{LibraryPlugin, LibraryPluginResource};
use rusqlite::{params, Result};

impl Database {
    pub fn insert_library_plugin(&self, plugin: &LibraryPlugin) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO library_plugins (id, name, description, category, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![plugin.id, plugin.name, plugin.description, plugin.category, plugin.created_at, plugin.updated_at],
        )?;
        Ok(())
    }

    pub fn get_library_plugin(&self, id: &str) -> Result<Option<LibraryPlugin>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, category, created_at, updated_at
             FROM library_plugins WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(LibraryPlugin {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                category: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn list_library_plugins(&self) -> Result<Vec<LibraryPlugin>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, category, created_at, updated_at
             FROM library_plugins ORDER BY name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(LibraryPlugin {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                category: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        let mut plugins = Vec::new();
        for row in rows {
            plugins.push(row?);
        }
        Ok(plugins)
    }

    pub fn delete_library_plugin(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM library_plugin_resources WHERE plugin_id = ?1", params![id])?;
        conn.execute("DELETE FROM library_plugins WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn add_resource_to_library_plugin(&self, link: &LibraryPluginResource) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO library_plugin_resources (id, plugin_id, resource_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![link.id, link.plugin_id, link.resource_id, link.created_at],
        )?;
        Ok(())
    }

    pub fn remove_resource_from_library_plugin(&self, plugin_id: &str, resource_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM library_plugin_resources WHERE plugin_id = ?1 AND resource_id = ?2",
            params![plugin_id, resource_id],
        )?;
        Ok(())
    }

    pub fn list_library_plugin_resources(&self, plugin_id: &str) -> Result<Vec<crate::models::v2::Resource>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT r.id, r.resource_type, r.name, r.description, r.scope, r.source_path, r.content_hash, r.metadata, r.created_at, r.updated_at, r.version, r.is_draft, r.installed_from_id
             FROM resources r
             INNER JOIN library_plugin_resources lpr ON r.id = lpr.resource_id
             WHERE lpr.plugin_id = ?1
             ORDER BY r.name"
        )?;
        let rows = stmt.query_map(params![plugin_id], |row| {
            use crate::models::v2::{ResourceType, ResourceScope};
            let type_str: String = row.get(1)?;
            let scope_str: String = row.get(4)?;
            Ok(crate::models::v2::Resource {
                id: row.get(0)?,
                resource_type: ResourceType::from_str(&type_str).unwrap_or(ResourceType::Skill),
                name: row.get(2)?,
                description: row.get(3)?,
                scope: ResourceScope::from_str(&scope_str).unwrap_or(ResourceScope::Library),
                source_path: row.get(5)?,
                content_hash: row.get(6)?,
                metadata: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                version: row.get(10)?,
                is_draft: row.get::<_, Option<i32>>(11)?.unwrap_or(1),
                installed_from_id: row.get(12)?,
            })
        })?;
        let mut resources = Vec::new();
        for row in rows {
            resources.push(row?);
        }
        Ok(resources)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_library_plugin(id: &str, name: &str) -> LibraryPlugin {
        LibraryPlugin {
            id: id.to_string(),
            name: name.to_string(),
            description: Some("Test plugin".to_string()),
            category: Some("development".to_string()),
            created_at: "2026-03-07T00:00:00Z".to_string(),
            updated_at: "2026-03-07T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_create_and_get_library_plugin() {
        let db = Database::new_in_memory().unwrap();
        let plugin = create_test_library_plugin("lp-1", "my-rules");
        db.insert_library_plugin(&plugin).unwrap();

        let fetched = db.get_library_plugin("lp-1").unwrap().unwrap();
        assert_eq!(fetched.name, "my-rules");
        assert_eq!(fetched.category, Some("development".to_string()));
    }

    #[test]
    fn test_list_library_plugins() {
        let db = Database::new_in_memory().unwrap();
        db.insert_library_plugin(&create_test_library_plugin("lp-1", "beta")).unwrap();
        db.insert_library_plugin(&create_test_library_plugin("lp-2", "alpha")).unwrap();

        let plugins = db.list_library_plugins().unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].name, "alpha");
    }

    #[test]
    fn test_delete_library_plugin() {
        let db = Database::new_in_memory().unwrap();
        db.insert_library_plugin(&create_test_library_plugin("lp-1", "test")).unwrap();
        assert!(db.get_library_plugin("lp-1").unwrap().is_some());

        db.delete_library_plugin("lp-1").unwrap();
        assert!(db.get_library_plugin("lp-1").unwrap().is_none());
    }

    #[test]
    fn test_add_and_list_plugin_resources() {
        let db = Database::new_in_memory().unwrap();
        db.insert_library_plugin(&create_test_library_plugin("lp-1", "test")).unwrap();

        use crate::models::v2::Resource;
        let resource = Resource {
            id: "r-1".to_string(),
            resource_type: crate::models::v2::ResourceType::Skill,
            name: "my-skill".to_string(),
            description: None,
            scope: crate::models::v2::ResourceScope::Library,
            source_path: "/tmp/skills/my-skill".to_string(),
            content_hash: None,
            metadata: None,
            created_at: "2026-03-07T00:00:00Z".to_string(),
            updated_at: "2026-03-07T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        let link = LibraryPluginResource {
            id: "lpr-1".to_string(),
            plugin_id: "lp-1".to_string(),
            resource_id: "r-1".to_string(),
            created_at: "2026-03-07T00:00:00Z".to_string(),
        };
        db.add_resource_to_library_plugin(&link).unwrap();

        let resources = db.list_library_plugin_resources("lp-1").unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "my-skill");
    }

    #[test]
    fn test_remove_resource_from_library_plugin() {
        let db = Database::new_in_memory().unwrap();
        db.insert_library_plugin(&create_test_library_plugin("lp-1", "test")).unwrap();

        use crate::models::v2::Resource;
        let resource = Resource {
            id: "r-1".to_string(),
            resource_type: crate::models::v2::ResourceType::Rule,
            name: "my-rule".to_string(),
            description: None,
            scope: crate::models::v2::ResourceScope::Library,
            source_path: "/tmp/rules/my-rule.md".to_string(),
            content_hash: None,
            metadata: None,
            created_at: "2026-03-07T00:00:00Z".to_string(),
            updated_at: "2026-03-07T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        };
        db.insert_resource(&resource).unwrap();

        let link = LibraryPluginResource {
            id: "lpr-1".to_string(),
            plugin_id: "lp-1".to_string(),
            resource_id: "r-1".to_string(),
            created_at: "2026-03-07T00:00:00Z".to_string(),
        };
        db.add_resource_to_library_plugin(&link).unwrap();
        assert_eq!(db.list_library_plugin_resources("lp-1").unwrap().len(), 1);

        db.remove_resource_from_library_plugin("lp-1", "r-1").unwrap();
        assert_eq!(db.list_library_plugin_resources("lp-1").unwrap().len(), 0);
    }
}
