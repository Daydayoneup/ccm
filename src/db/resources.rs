use rusqlite::params;

use crate::db::Database;
use crate::models::v2::{Resource, ResourceScope, ResourceType};

const RESOURCE_COLUMNS: &str = "id, resource_type, name, description, scope, source_path, content_hash, metadata, created_at, updated_at, version, is_draft, installed_from_id";

impl Database {
    pub fn insert_resource(&self, resource: &Resource) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO resources (id, resource_type, name, description, scope, source_path, content_hash, metadata, created_at, updated_at, version, is_draft, installed_from_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                resource.id,
                resource.resource_type.as_str(),
                resource.name,
                resource.description,
                resource.scope.as_str(),
                resource.source_path,
                resource.content_hash,
                resource.metadata,
                resource.created_at,
                resource.updated_at,
                resource.version,
                resource.is_draft,
                resource.installed_from_id,
            ],
        )?;
        Ok(())
    }

    pub fn get_resource(&self, id: &str) -> rusqlite::Result<Option<Resource>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM resources WHERE id = ?1", RESOURCE_COLUMNS)
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Self::row_to_resource(row)
        })?;
        match rows.next() {
            Some(result) => Ok(Some(result?)),
            None => Ok(None),
        }
    }

    pub fn list_resources_by_scope(&self, scope: &ResourceScope) -> rusqlite::Result<Vec<Resource>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM resources WHERE scope = ?1", RESOURCE_COLUMNS)
        )?;
        let rows = stmt.query_map(params![scope.as_str()], |row| {
            Self::row_to_resource(row)
        })?;
        rows.collect()
    }

    pub fn list_resources_by_scope_and_type(
        &self,
        scope: &ResourceScope,
        resource_type: &ResourceType,
    ) -> rusqlite::Result<Vec<Resource>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM resources WHERE scope = ?1 AND resource_type = ?2", RESOURCE_COLUMNS)
        )?;
        let rows = stmt.query_map(params![scope.as_str(), resource_type.as_str()], |row| {
            Self::row_to_resource(row)
        })?;
        rows.collect()
    }

    pub fn update_resource(&self, resource: &Resource) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE resources SET resource_type = ?1, name = ?2, description = ?3, content_hash = ?4, metadata = ?5, updated_at = ?6, source_path = ?7, scope = ?8, version = ?9, is_draft = ?10, installed_from_id = ?11 WHERE id = ?12",
            params![
                resource.resource_type.as_str(),
                resource.name,
                resource.description,
                resource.content_hash,
                resource.metadata,
                resource.updated_at,
                resource.source_path,
                resource.scope.as_str(),
                resource.version,
                resource.is_draft,
                resource.installed_from_id,
                resource.id,
            ],
        )?;
        Ok(())
    }

    pub fn delete_resource(&self, id: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM resources WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Delete all registry-scoped resources whose source_path starts with the given prefix.
    /// This catches orphaned resources that may not be linked to any current registry_plugin.
    pub fn delete_registry_resources_by_path_prefix(&self, path_prefix: &str) -> rusqlite::Result<usize> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("{}%", path_prefix);
        conn.execute(
            "DELETE FROM resources WHERE scope = 'registry' AND source_path LIKE ?1",
            params![pattern],
        )
    }

    /// List all registry-scoped resources whose source_path starts with the given prefix.
    pub fn list_registry_resources_by_path_prefix(&self, path_prefix: &str) -> rusqlite::Result<Vec<Resource>> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("{}%", path_prefix);
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM resources WHERE scope = 'registry' AND source_path LIKE ?1", RESOURCE_COLUMNS)
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            Self::row_to_resource(row)
        })?;
        rows.collect()
    }

    pub fn get_resource_by_path(&self, source_path: &str) -> rusqlite::Result<Option<Resource>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM resources WHERE source_path = ?1", RESOURCE_COLUMNS)
        )?;
        let mut rows = stmt.query_map(params![source_path], |row| {
            Self::row_to_resource(row)
        })?;
        match rows.next() {
            Some(result) => Ok(Some(result?)),
            None => Ok(None),
        }
    }

    /// List Registry/Library resources whose source_path starts with the given prefix.
    /// Used to find resources installed into ~/.claude/ via symlink from registry or library.
    pub fn list_managed_resources_by_path_prefix(
        &self,
        path_prefix: &str,
        resource_type: Option<&ResourceType>,
    ) -> rusqlite::Result<Vec<Resource>> {
        let conn = self.conn.lock().unwrap();
        let prefix_pattern = format!("{}%", path_prefix);
        let resources = if let Some(rt) = resource_type {
            let mut stmt = conn.prepare(
                &format!("SELECT {} FROM resources WHERE source_path LIKE ?1 AND scope IN ('registry', 'library') AND resource_type = ?2", RESOURCE_COLUMNS)
            )?;
            let rows = stmt.query_map(params![prefix_pattern, rt.as_str()], |row| {
                Self::row_to_resource(row)
            })?;
            rows.collect()
        } else {
            let mut stmt = conn.prepare(
                &format!("SELECT {} FROM resources WHERE source_path LIKE ?1 AND scope IN ('registry', 'library')", RESOURCE_COLUMNS)
            )?;
            let rows = stmt.query_map(params![prefix_pattern], |row| {
                Self::row_to_resource(row)
            })?;
            rows.collect()
        };
        resources
    }

    pub fn count_resources_by_scope(&self, scope: &ResourceScope) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM resources WHERE scope = ?1",
            params![scope.as_str()],
            |row| row.get(0),
        )
    }

    pub fn list_recent_resources(&self, limit: usize) -> rusqlite::Result<Vec<Resource>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM resources ORDER BY updated_at DESC LIMIT ?1", RESOURCE_COLUMNS)
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Self::row_to_resource(row)
        })?;
        rows.collect()
    }

    pub fn search_resources(&self, query: &str) -> rusqlite::Result<Vec<Resource>> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM resources WHERE name LIKE ?1 ORDER BY updated_at DESC LIMIT 50", RESOURCE_COLUMNS)
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            Self::row_to_resource(row)
        })?;
        rows.collect()
    }

    fn row_to_resource(row: &rusqlite::Row) -> rusqlite::Result<Resource> {
        let resource_type_str: String = row.get(1)?;
        let scope_str: String = row.get(4)?;
        Ok(Resource {
            id: row.get(0)?,
            resource_type: ResourceType::from_str(&resource_type_str).unwrap_or(ResourceType::Skill),
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
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::models::v2::{Resource, ResourceScope, ResourceType};

    fn make_resource(id: &str, scope: ResourceScope) -> Resource {
        Resource {
            id: id.to_string(),
            resource_type: ResourceType::Skill,
            name: format!("test-{}", id),
            description: None,
            scope,
            source_path: format!("/tmp/{}", id),
            content_hash: Some("abc123".to_string()),
            metadata: None,
            created_at: "2026-03-01T00:00:00Z".to_string(),
            updated_at: "2026-03-01T00:00:00Z".to_string(),
            version: None,
            is_draft: 1,
            installed_from_id: None,
        }
    }

    #[test]
    fn test_insert_and_get_resource() {
        let db = Database::new_in_memory().unwrap();
        let resource = make_resource("r1", ResourceScope::Library);

        db.insert_resource(&resource).unwrap();

        let fetched = db.get_resource("r1").unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, "r1");
        assert_eq!(fetched.name, "test-r1");
        assert_eq!(fetched.resource_type.as_str(), "skill");
        assert_eq!(fetched.scope.as_str(), "library");
        assert_eq!(fetched.source_path, "/tmp/r1");
        assert_eq!(fetched.content_hash, Some("abc123".to_string()));
        assert_eq!(fetched.metadata, None);

        // Non-existent resource returns None
        let missing = db.get_resource("nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_list_resources_by_scope() {
        let db = Database::new_in_memory().unwrap();
        let r1 = make_resource("r1", ResourceScope::Library);
        let r2 = make_resource("r2", ResourceScope::Library);
        let r3 = make_resource("r3", ResourceScope::Global);

        db.insert_resource(&r1).unwrap();
        db.insert_resource(&r2).unwrap();
        db.insert_resource(&r3).unwrap();

        let library_resources = db.list_resources_by_scope(&ResourceScope::Library).unwrap();
        assert_eq!(library_resources.len(), 2);

        let global_resources = db.list_resources_by_scope(&ResourceScope::Global).unwrap();
        assert_eq!(global_resources.len(), 1);
        assert_eq!(global_resources[0].id, "r3");

        let project_resources = db.list_resources_by_scope(&ResourceScope::Project).unwrap();
        assert_eq!(project_resources.len(), 0);
    }

    #[test]
    fn test_list_resources_by_scope_and_type() {
        let db = Database::new_in_memory().unwrap();
        let mut r1 = make_resource("r1", ResourceScope::Library);
        r1.resource_type = ResourceType::Skill;

        let mut r2 = make_resource("r2", ResourceScope::Library);
        r2.resource_type = ResourceType::Agent;

        let mut r3 = make_resource("r3", ResourceScope::Library);
        r3.resource_type = ResourceType::Skill;

        let mut r4 = make_resource("r4", ResourceScope::Global);
        r4.resource_type = ResourceType::Skill;

        db.insert_resource(&r1).unwrap();
        db.insert_resource(&r2).unwrap();
        db.insert_resource(&r3).unwrap();
        db.insert_resource(&r4).unwrap();

        let library_skills = db
            .list_resources_by_scope_and_type(&ResourceScope::Library, &ResourceType::Skill)
            .unwrap();
        assert_eq!(library_skills.len(), 2);

        let library_agents = db
            .list_resources_by_scope_and_type(&ResourceScope::Library, &ResourceType::Agent)
            .unwrap();
        assert_eq!(library_agents.len(), 1);
        assert_eq!(library_agents[0].id, "r2");

        let library_hooks = db
            .list_resources_by_scope_and_type(&ResourceScope::Library, &ResourceType::Hook)
            .unwrap();
        assert_eq!(library_hooks.len(), 0);
    }

    #[test]
    fn test_update_resource() {
        let db = Database::new_in_memory().unwrap();
        let resource = make_resource("r1", ResourceScope::Library);
        db.insert_resource(&resource).unwrap();

        let mut updated = resource.clone();
        updated.name = "updated-name".to_string();
        updated.description = Some("a description".to_string());
        updated.resource_type = ResourceType::Agent;
        updated.content_hash = Some("newhash".to_string());
        updated.updated_at = "2026-03-02T00:00:00Z".to_string();

        db.update_resource(&updated).unwrap();

        let fetched = db.get_resource("r1").unwrap().unwrap();
        assert_eq!(fetched.name, "updated-name");
        assert_eq!(fetched.description, Some("a description".to_string()));
        assert_eq!(fetched.resource_type.as_str(), "agent");
        assert_eq!(fetched.content_hash, Some("newhash".to_string()));
        assert_eq!(fetched.updated_at, "2026-03-02T00:00:00Z");
        // created_at should remain unchanged
        assert_eq!(fetched.created_at, "2026-03-01T00:00:00Z");
    }

    #[test]
    fn test_delete_resource() {
        let db = Database::new_in_memory().unwrap();
        let resource = make_resource("r1", ResourceScope::Library);
        db.insert_resource(&resource).unwrap();

        // Verify it exists
        assert!(db.get_resource("r1").unwrap().is_some());

        db.delete_resource("r1").unwrap();

        // Verify it's gone
        assert!(db.get_resource("r1").unwrap().is_none());

        // Deleting a non-existent resource should not error
        db.delete_resource("nonexistent").unwrap();
    }

    #[test]
    fn test_get_resource_by_path() {
        let db = Database::new_in_memory().unwrap();
        let resource = make_resource("r1", ResourceScope::Library);
        db.insert_resource(&resource).unwrap();

        let fetched = db.get_resource_by_path("/tmp/r1").unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, "r1");

        let missing = db.get_resource_by_path("/tmp/nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_count_resources_by_scope() {
        let db = Database::new_in_memory().unwrap();

        // Empty count
        let count = db.count_resources_by_scope(&ResourceScope::Library).unwrap();
        assert_eq!(count, 0);

        db.insert_resource(&make_resource("r1", ResourceScope::Library)).unwrap();
        db.insert_resource(&make_resource("r2", ResourceScope::Library)).unwrap();
        db.insert_resource(&make_resource("r3", ResourceScope::Global)).unwrap();

        let library_count = db.count_resources_by_scope(&ResourceScope::Library).unwrap();
        assert_eq!(library_count, 2);

        let global_count = db.count_resources_by_scope(&ResourceScope::Global).unwrap();
        assert_eq!(global_count, 1);

        let project_count = db.count_resources_by_scope(&ResourceScope::Project).unwrap();
        assert_eq!(project_count, 0);
    }

    #[test]
    fn test_list_recent_resources() {
        let db = Database::new_in_memory().unwrap();

        // Empty list
        let recent = db.list_recent_resources(5).unwrap();
        assert_eq!(recent.len(), 0);

        // Insert resources with different updated_at timestamps
        let mut r1 = make_resource("r1", ResourceScope::Library);
        r1.updated_at = "2026-03-01T00:00:00Z".to_string();
        let mut r2 = make_resource("r2", ResourceScope::Global);
        r2.updated_at = "2026-03-02T00:00:00Z".to_string();
        let mut r3 = make_resource("r3", ResourceScope::Project);
        r3.updated_at = "2026-03-03T00:00:00Z".to_string();

        db.insert_resource(&r1).unwrap();
        db.insert_resource(&r2).unwrap();
        db.insert_resource(&r3).unwrap();

        // Get all 3
        let recent = db.list_recent_resources(10).unwrap();
        assert_eq!(recent.len(), 3);
        // Most recent first
        assert_eq!(recent[0].id, "r3");
        assert_eq!(recent[1].id, "r2");
        assert_eq!(recent[2].id, "r1");

        // Limit to 2
        let recent = db.list_recent_resources(2).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, "r3");
        assert_eq!(recent[1].id, "r2");
    }

    #[test]
    fn test_search_resources() {
        let db = Database::new_in_memory().unwrap();

        // Empty search
        let results = db.search_resources("anything").unwrap();
        assert_eq!(results.len(), 0);

        let mut r1 = make_resource("r1", ResourceScope::Library);
        r1.name = "my-cool-skill".to_string();
        let mut r2 = make_resource("r2", ResourceScope::Global);
        r2.name = "another-skill".to_string();
        let mut r3 = make_resource("r3", ResourceScope::Project);
        r3.name = "hook-linter".to_string();

        db.insert_resource(&r1).unwrap();
        db.insert_resource(&r2).unwrap();
        db.insert_resource(&r3).unwrap();

        // Search for "skill" should match r1 and r2
        let results = db.search_resources("skill").unwrap();
        assert_eq!(results.len(), 2);

        // Search for "cool" should match r1 only
        let results = db.search_resources("cool").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "r1");

        // Search for "hook" should match r3
        let results = db.search_resources("hook").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "r3");

        // Search for "nonexistent" should return empty
        let results = db.search_resources("nonexistent").unwrap();
        assert_eq!(results.len(), 0);
    }
}
