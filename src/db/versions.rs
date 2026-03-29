use rusqlite::params;

use crate::db::Database;
use crate::models::v2::ResourceVersion;

impl Database {
    pub fn insert_resource_version(&self, version: &ResourceVersion) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO resource_versions (id, resource_id, version, changelog, content_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                version.id,
                version.resource_id,
                version.version,
                version.changelog,
                version.content_hash,
                version.created_at,
            ],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                format!(
                    "Version '{}' already exists for resource '{}'",
                    version.version, version.resource_id
                )
            } else {
                e.to_string()
            }
        })?;
        Ok(())
    }

    pub fn list_resource_versions(&self, resource_id: &str) -> Result<Vec<ResourceVersion>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, resource_id, version, changelog, content_hash, created_at
                 FROM resource_versions
                 WHERE resource_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![resource_id], |row| {
                Ok(ResourceVersion {
                    id: row.get(0)?,
                    resource_id: row.get(1)?,
                    version: row.get(2)?,
                    changelog: row.get(3)?,
                    content_hash: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
    }

    pub fn get_resource_version(
        &self,
        resource_id: &str,
        version: &str,
    ) -> Result<Option<ResourceVersion>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, resource_id, version, changelog, content_hash, created_at
                 FROM resource_versions
                 WHERE resource_id = ?1 AND version = ?2",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![resource_id, version], |row| {
                Ok(ResourceVersion {
                    id: row.get(0)?,
                    resource_id: row.get(1)?,
                    version: row.get(2)?,
                    changelog: row.get(3)?,
                    content_hash: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        match rows.next() {
            Some(result) => Ok(Some(result.map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }

    pub fn delete_resource_versions_by_resource(&self, resource_id: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM resource_versions WHERE resource_id = ?1",
            params![resource_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Database {
        let db = Database::new_in_memory().unwrap();
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO resources (id, resource_type, name, scope, source_path, created_at, updated_at, is_draft)
             VALUES ('r1', 'skill', 'test-skill', 'library', '/tmp/test', '2026-01-01', '2026-01-01', 1)",
            [],
        ).unwrap();
        drop(conn);
        db
    }

    #[test]
    fn test_insert_and_list_versions() {
        let db = setup_db();
        let v = ResourceVersion {
            id: "v1".to_string(),
            resource_id: "r1".to_string(),
            version: "1.0.0".to_string(),
            changelog: Some("Initial release".to_string()),
            content_hash: "abc123".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        db.insert_resource_version(&v).unwrap();

        let versions = db.list_resource_versions("r1").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].id, "v1");
        assert_eq!(versions[0].resource_id, "r1");
        assert_eq!(versions[0].version, "1.0.0");
        assert_eq!(versions[0].changelog, Some("Initial release".to_string()));
        assert_eq!(versions[0].content_hash, "abc123");
        assert_eq!(versions[0].created_at, "2026-01-01T00:00:00Z");
    }

    #[test]
    fn test_duplicate_version_fails() {
        let db = setup_db();
        let v = ResourceVersion {
            id: "v1".to_string(),
            resource_id: "r1".to_string(),
            version: "1.0.0".to_string(),
            changelog: None,
            content_hash: "abc123".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        db.insert_resource_version(&v).unwrap();

        let v2 = ResourceVersion {
            id: "v2".to_string(),
            resource_id: "r1".to_string(),
            version: "1.0.0".to_string(), // same version for same resource
            changelog: None,
            content_hash: "def456".to_string(),
            created_at: "2026-01-02T00:00:00Z".to_string(),
        };
        let result = db.insert_resource_version(&v2);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("already exists"),
            "Expected friendly duplicate message, got: {err}"
        );
    }

    #[test]
    fn test_list_versions_ordered_by_created_at_desc() {
        let db = setup_db();

        let versions = vec![
            ResourceVersion {
                id: "v1".to_string(),
                resource_id: "r1".to_string(),
                version: "1.0.0".to_string(),
                changelog: None,
                content_hash: "hash1".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
            },
            ResourceVersion {
                id: "v2".to_string(),
                resource_id: "r1".to_string(),
                version: "1.1.0".to_string(),
                changelog: None,
                content_hash: "hash2".to_string(),
                created_at: "2026-02-01T00:00:00Z".to_string(),
            },
            ResourceVersion {
                id: "v3".to_string(),
                resource_id: "r1".to_string(),
                version: "2.0.0".to_string(),
                changelog: None,
                content_hash: "hash3".to_string(),
                created_at: "2026-03-01T00:00:00Z".to_string(),
            },
        ];

        for v in &versions {
            db.insert_resource_version(v).unwrap();
        }

        let listed = db.list_resource_versions("r1").unwrap();
        assert_eq!(listed.len(), 3);
        // Should be ordered DESC by created_at: v3, v2, v1
        assert_eq!(listed[0].version, "2.0.0");
        assert_eq!(listed[1].version, "1.1.0");
        assert_eq!(listed[2].version, "1.0.0");
    }

    #[test]
    fn test_get_resource_version() {
        let db = setup_db();
        let v = ResourceVersion {
            id: "v1".to_string(),
            resource_id: "r1".to_string(),
            version: "1.0.0".to_string(),
            changelog: Some("First version".to_string()),
            content_hash: "abc123".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        db.insert_resource_version(&v).unwrap();

        // Found case
        let found = db.get_resource_version("r1", "1.0.0").unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, "v1");
        assert_eq!(found.version, "1.0.0");
        assert_eq!(found.changelog, Some("First version".to_string()));

        // Not found case
        let not_found = db.get_resource_version("r1", "9.9.9").unwrap();
        assert!(not_found.is_none());

        // Not found — wrong resource_id
        let not_found2 = db.get_resource_version("r_nonexistent", "1.0.0").unwrap();
        assert!(not_found2.is_none());
    }
}
