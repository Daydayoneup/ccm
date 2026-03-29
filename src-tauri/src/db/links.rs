use crate::db::Database;
use crate::models::v2::ResourceLink;
use rusqlite::{params, Result};

const LINK_COLUMNS: &str = "id, resource_id, target_scope, target_path, config_key, project_id, link_type, created_at, installed_hash";

impl Database {
    pub fn insert_link(&self, link: &ResourceLink) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO resource_links (id, resource_id, target_scope, target_path, config_key, project_id, link_type, created_at, installed_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                link.id,
                link.resource_id,
                link.target_scope,
                link.target_path,
                link.config_key,
                link.project_id,
                link.link_type,
                link.created_at,
                link.installed_hash,
            ],
        )?;
        Ok(())
    }

    pub fn get_link(&self, id: &str) -> Result<Option<ResourceLink>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM resource_links WHERE id = ?1", LINK_COLUMNS)
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(ResourceLink {
                id: row.get(0)?,
                resource_id: row.get(1)?,
                target_scope: row.get(2)?,
                target_path: row.get(3)?,
                config_key: row.get(4)?,
                project_id: row.get(5)?,
                link_type: row.get(6)?,
                created_at: row.get(7)?,
                installed_hash: row.get(8)?,
            })
        })?;
        match rows.next() {
            Some(result) => Ok(Some(result?)),
            None => Ok(None),
        }
    }

    pub fn list_links_by_resource(&self, resource_id: &str) -> Result<Vec<ResourceLink>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM resource_links WHERE resource_id = ?1", LINK_COLUMNS)
        )?;
        let rows = stmt.query_map(params![resource_id], |row| {
            Ok(ResourceLink {
                id: row.get(0)?,
                resource_id: row.get(1)?,
                target_scope: row.get(2)?,
                target_path: row.get(3)?,
                config_key: row.get(4)?,
                project_id: row.get(5)?,
                link_type: row.get(6)?,
                created_at: row.get(7)?,
                installed_hash: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_links_by_project(&self, project_id: &str) -> Result<Vec<ResourceLink>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM resource_links WHERE project_id = ?1", LINK_COLUMNS)
        )?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok(ResourceLink {
                id: row.get(0)?,
                resource_id: row.get(1)?,
                target_scope: row.get(2)?,
                target_path: row.get(3)?,
                config_key: row.get(4)?,
                project_id: row.get(5)?,
                link_type: row.get(6)?,
                created_at: row.get(7)?,
                installed_hash: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_global_links(&self) -> Result<Vec<ResourceLink>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM resource_links WHERE project_id IS NULL AND target_scope = 'global'", LINK_COLUMNS)
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ResourceLink {
                id: row.get(0)?,
                resource_id: row.get(1)?,
                target_scope: row.get(2)?,
                target_path: row.get(3)?,
                config_key: row.get(4)?,
                project_id: row.get(5)?,
                link_type: row.get(6)?,
                created_at: row.get(7)?,
                installed_hash: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_all_links(&self) -> Result<Vec<ResourceLink>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM resource_links", LINK_COLUMNS)
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ResourceLink {
                id: row.get(0)?,
                resource_id: row.get(1)?,
                target_scope: row.get(2)?,
                target_path: row.get(3)?,
                config_key: row.get(4)?,
                project_id: row.get(5)?,
                link_type: row.get(6)?,
                created_at: row.get(7)?,
                installed_hash: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn update_link_installed_hash(&self, link_id: &str, installed_hash: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE resource_links SET installed_hash = ?1 WHERE id = ?2",
            params![installed_hash, link_id],
        )?;
        Ok(())
    }

    pub fn delete_link(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM resource_links WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_link_by_target_path(&self, target_path: &str) -> Result<Option<ResourceLink>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM resource_links WHERE target_path = ?1", LINK_COLUMNS)
        )?;
        let mut rows = stmt.query_map(params![target_path], |row| {
            Ok(ResourceLink {
                id: row.get(0)?,
                resource_id: row.get(1)?,
                target_scope: row.get(2)?,
                target_path: row.get(3)?,
                config_key: row.get(4)?,
                project_id: row.get(5)?,
                link_type: row.get(6)?,
                created_at: row.get(7)?,
                installed_hash: row.get(8)?,
            })
        })?;
        match rows.next() {
            Some(result) => Ok(Some(result?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::models::v2::ResourceLink;

    fn setup_test_data(db: &Database) {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO resources (id, resource_type, name, scope, source_path, created_at, updated_at) VALUES ('res1', 'skill', 'test-skill', 'library', '/tmp/skill', '2026-03-01', '2026-03-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, path) VALUES ('proj1', 'test-project', '/tmp/proj1')",
            [],
        )
        .unwrap();
    }

    fn make_link(id: &str, project_id: Option<String>) -> ResourceLink {
        ResourceLink {
            id: id.to_string(),
            resource_id: "res1".to_string(),
            target_scope: if project_id.is_some() {
                "project".to_string()
            } else {
                "global".to_string()
            },
            target_path: format!("/target/{}", id),
            config_key: None,
            project_id,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: None,
        }
    }

    #[test]
    fn test_insert_and_get_link() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        let link = make_link("link1", None);
        db.insert_link(&link).unwrap();

        let fetched = db.get_link("link1").unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, "link1");
        assert_eq!(fetched.resource_id, "res1");
        assert_eq!(fetched.target_scope, "global");
        assert_eq!(fetched.target_path, "/target/link1");
        assert_eq!(fetched.project_id, None);
        assert_eq!(fetched.link_type, "symlink");
        assert_eq!(fetched.created_at, "2026-03-01T00:00:00Z");
    }

    #[test]
    fn test_get_nonexistent_link() {
        let db = Database::new_in_memory().unwrap();
        let result = db.get_link("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_insert_link_with_project() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        let link = make_link("link1", Some("proj1".to_string()));
        db.insert_link(&link).unwrap();

        let fetched = db.get_link("link1").unwrap().unwrap();
        assert_eq!(fetched.project_id, Some("proj1".to_string()));
        assert_eq!(fetched.target_scope, "project");
    }

    #[test]
    fn test_insert_duplicate_id_fails() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        let link = make_link("link1", None);
        db.insert_link(&link).unwrap();

        let dup = make_link("link1", None);
        let result = db.insert_link(&dup);
        assert!(result.is_err());
    }

    #[test]
    fn test_insert_with_invalid_resource_id_fk_fails() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        let mut link = make_link("link1", None);
        link.resource_id = "nonexistent_resource".to_string();
        let result = db.insert_link(&link);
        assert!(result.is_err());
    }

    #[test]
    fn test_insert_with_invalid_project_id_fk_fails() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        let link = make_link("link1", Some("nonexistent_project".to_string()));
        let result = db.insert_link(&link);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_links_by_resource() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        // Insert a second resource for isolation testing
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO resources (id, resource_type, name, scope, source_path, created_at, updated_at) VALUES ('res2', 'agent', 'test-agent', 'library', '/tmp/agent', '2026-03-01', '2026-03-01')",
                [],
            )
            .unwrap();
        }

        let link1 = make_link("link1", None);
        let link2 = make_link("link2", Some("proj1".to_string()));
        let mut link3 = make_link("link3", None);
        link3.resource_id = "res2".to_string();

        db.insert_link(&link1).unwrap();
        db.insert_link(&link2).unwrap();
        db.insert_link(&link3).unwrap();

        let res1_links = db.list_links_by_resource("res1").unwrap();
        assert_eq!(res1_links.len(), 2);
        assert!(res1_links.iter().all(|l| l.resource_id == "res1"));

        let res2_links = db.list_links_by_resource("res2").unwrap();
        assert_eq!(res2_links.len(), 1);
        assert_eq!(res2_links[0].id, "link3");

        let empty = db.list_links_by_resource("nonexistent").unwrap();
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_list_links_by_project() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        // Add a second project
        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO projects (id, name, path) VALUES ('proj2', 'test-project-2', '/tmp/proj2')",
                [],
            )
            .unwrap();
        }

        let link1 = make_link("link1", Some("proj1".to_string()));
        let link2 = make_link("link2", Some("proj1".to_string()));
        let link3 = make_link("link3", Some("proj2".to_string()));
        let link4 = make_link("link4", None); // global, no project

        db.insert_link(&link1).unwrap();
        db.insert_link(&link2).unwrap();
        db.insert_link(&link3).unwrap();
        db.insert_link(&link4).unwrap();

        let proj1_links = db.list_links_by_project("proj1").unwrap();
        assert_eq!(proj1_links.len(), 2);
        assert!(proj1_links
            .iter()
            .all(|l| l.project_id == Some("proj1".to_string())));

        let proj2_links = db.list_links_by_project("proj2").unwrap();
        assert_eq!(proj2_links.len(), 1);
        assert_eq!(proj2_links[0].id, "link3");

        let empty = db.list_links_by_project("nonexistent").unwrap();
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_list_global_links() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        let link1 = make_link("link1", None); // global scope, no project
        let link2 = make_link("link2", Some("proj1".to_string())); // project scope, has project
        let link3 = make_link("link3", None); // global scope, no project

        db.insert_link(&link1).unwrap();
        db.insert_link(&link2).unwrap();
        db.insert_link(&link3).unwrap();

        let global = db.list_global_links().unwrap();
        assert_eq!(global.len(), 2);
        assert!(global
            .iter()
            .all(|l| l.project_id.is_none() && l.target_scope == "global"));
    }

    #[test]
    fn test_list_global_links_excludes_non_global_scope() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        let link1 = make_link("link1", None); // global scope, no project

        // Manually create a link with project_id=NULL but target_scope != 'global'
        let link2 = ResourceLink {
            id: "link2".to_string(),
            resource_id: "res1".to_string(),
            target_scope: "library".to_string(),
            target_path: "/target/link2".to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: None,
        };

        db.insert_link(&link1).unwrap();
        db.insert_link(&link2).unwrap();

        let global = db.list_global_links().unwrap();
        assert_eq!(global.len(), 1);
        assert_eq!(global[0].id, "link1");
    }

    #[test]
    fn test_delete_link() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        let link = make_link("link1", None);
        db.insert_link(&link).unwrap();

        assert!(db.get_link("link1").unwrap().is_some());

        db.delete_link("link1").unwrap();
        assert!(db.get_link("link1").unwrap().is_none());
    }

    #[test]
    fn test_delete_nonexistent_link() {
        let db = Database::new_in_memory().unwrap();
        // Should not error even if no rows are deleted
        db.delete_link("nonexistent").unwrap();
    }

    #[test]
    fn test_get_link_by_target_path() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        let link = make_link("link1", None);
        db.insert_link(&link).unwrap();

        let fetched = db.get_link_by_target_path("/target/link1").unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, "link1");
        assert_eq!(fetched.target_path, "/target/link1");
    }

    #[test]
    fn test_get_link_by_target_path_not_found() {
        let db = Database::new_in_memory().unwrap();
        let result = db.get_link_by_target_path("/nonexistent/path").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_cascade_delete_on_resource() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        let link = make_link("link1", None);
        db.insert_link(&link).unwrap();
        assert!(db.get_link("link1").unwrap().is_some());

        // Delete the parent resource; the link should be cascade-deleted
        {
            let conn = db.conn.lock().unwrap();
            conn.execute("DELETE FROM resources WHERE id = 'res1'", [])
                .unwrap();
        }

        assert!(db.get_link("link1").unwrap().is_none());
    }

    #[test]
    fn test_insert_and_get_link_with_config_key() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        let link = ResourceLink {
            id: "link-cfg".to_string(),
            resource_id: "res1".to_string(),
            target_scope: "project".to_string(),
            target_path: "/target/link-cfg".to_string(),
            config_key: Some("mcpServers.my-server".to_string()),
            project_id: Some("proj1".to_string()),
            link_type: "config".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: None,
        };
        db.insert_link(&link).unwrap();

        let fetched = db.get_link("link-cfg").unwrap().unwrap();
        assert_eq!(fetched.config_key, Some("mcpServers.my-server".to_string()));
        assert_eq!(fetched.link_type, "config");

        // Also verify a NULL config_key round-trips correctly
        let link2 = make_link("link-no-cfg", None);
        db.insert_link(&link2).unwrap();
        let fetched2 = db.get_link("link-no-cfg").unwrap().unwrap();
        assert_eq!(fetched2.config_key, None);
    }

    #[test]
    fn test_cascade_delete_on_project() {
        let db = Database::new_in_memory().unwrap();
        setup_test_data(&db);

        let link = make_link("link1", Some("proj1".to_string()));
        db.insert_link(&link).unwrap();
        assert!(db.get_link("link1").unwrap().is_some());

        // Delete the parent project; the link should be cascade-deleted
        {
            let conn = db.conn.lock().unwrap();
            conn.execute("DELETE FROM projects WHERE id = 'proj1'", [])
                .unwrap();
        }

        assert!(db.get_link("link1").unwrap().is_none());
    }

    #[test]
    fn test_insert_link_with_installed_hash() {
        let db = Database::new_in_memory().unwrap();
        db.migrate_v7_to_v8().unwrap();
        setup_test_data(&db);

        let link = ResourceLink {
            id: "link-hash".to_string(),
            resource_id: "res1".to_string(),
            target_scope: "global".to_string(),
            target_path: "/target/link-hash".to_string(),
            config_key: None,
            project_id: None,
            link_type: "symlink".to_string(),
            created_at: "2026-03-01T00:00:00Z".to_string(),
            installed_hash: Some("abc123hash".to_string()),
        };
        db.insert_link(&link).unwrap();

        let fetched = db.get_link("link-hash").unwrap().unwrap();
        assert_eq!(fetched.installed_hash, Some("abc123hash".to_string()));
    }

    #[test]
    fn test_update_link_installed_hash() {
        let db = Database::new_in_memory().unwrap();
        db.migrate_v7_to_v8().unwrap();
        setup_test_data(&db);

        let link = make_link("link1", None);
        db.insert_link(&link).unwrap();

        // Initially None
        let fetched = db.get_link("link1").unwrap().unwrap();
        assert_eq!(fetched.installed_hash, None);

        // Update to a value
        db.update_link_installed_hash("link1", Some("newhash")).unwrap();
        let fetched = db.get_link("link1").unwrap().unwrap();
        assert_eq!(fetched.installed_hash, Some("newhash".to_string()));

        // Clear it back to None
        db.update_link_installed_hash("link1", None).unwrap();
        let fetched = db.get_link("link1").unwrap().unwrap();
        assert_eq!(fetched.installed_hash, None);
    }
}
