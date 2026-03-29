pub mod watcher;
pub mod state;

use crate::db::Database;
use crate::scanner::ScannedResource;
use crate::models::v2::{Resource, ResourceScope};
use std::collections::HashMap;

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct SyncReport {
    pub inserted: usize,
    pub updated: usize,
    pub deleted: usize,
}

pub struct SyncEngine;

impl SyncEngine {
    /// Full reconciliation: compare scanned files vs DB records for a given scope.
    /// Uses `source_path::name` as composite key so multiple resources from
    /// the same file (e.g. MCP servers in one .mcp.json) are tracked individually.
    pub fn reconcile(
        db: &Database,
        scope: &ResourceScope,
        scanned: Vec<ScannedResource>,
    ) -> Result<SyncReport, String> {
        let db_resources = db.list_resources_by_scope(scope)
            .map_err(|e| e.to_string())?;

        let mut db_map: HashMap<String, Resource> = db_resources
            .into_iter()
            .map(|r| (format!("{}::{}", r.source_path, r.name), r))
            .collect();

        let mut report = SyncReport::default();

        for scanned_res in scanned {
            let effective_scope = scanned_res.scope_override.as_ref().unwrap_or(scope);
            let key = format!("{}::{}", scanned_res.source_path, scanned_res.name);
            if let Some(existing) = db_map.remove(&key) {
                // File exists in both FS and DB — check hash, scope, or metadata change
                let scope_changed = existing.scope != *effective_scope;
                let metadata_changed = scanned_res.linked_metadata != existing.metadata;
                let installed_from_changed = scanned_res.installed_from_id != existing.installed_from_id;
                if scanned_res.content_hash != existing.content_hash || scope_changed || metadata_changed || installed_from_changed {
                    let mut updated = existing;
                    updated.content_hash = scanned_res.content_hash;
                    updated.scope = effective_scope.clone();
                    updated.metadata = scanned_res.linked_metadata;
                    updated.installed_from_id = scanned_res.installed_from_id;
                    updated.updated_at = chrono::Utc::now().to_rfc3339();
                    db.update_resource(&updated).map_err(|e| e.to_string())?;
                    report.updated += 1;
                }
            } else {
                // File in FS but not in DB — insert
                let now = chrono::Utc::now().to_rfc3339();
                let resource = Resource {
                    id: uuid::Uuid::new_v4().to_string(),
                    resource_type: scanned_res.resource_type,
                    name: scanned_res.name,
                    description: None,
                    scope: effective_scope.clone(),
                    source_path: scanned_res.source_path,
                    content_hash: scanned_res.content_hash,
                    metadata: scanned_res.linked_metadata,
                    installed_from_id: scanned_res.installed_from_id,
                    created_at: now.clone(),
                    updated_at: now,
                    version: None,
                    is_draft: 1,
                };
                db.insert_resource(&resource).map_err(|e| e.to_string())?;
                report.inserted += 1;
            }
        }

        // Remaining in db_map = in DB but not on FS — deleted externally
        for (_, resource) in db_map {
            db.delete_resource(&resource.id).map_err(|e| e.to_string())?;
            report.deleted += 1;
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::v2::ResourceType;

    fn make_scanned(name: &str, path: &str, hash: &str) -> ScannedResource {
        ScannedResource {
            resource_type: ResourceType::Skill,
            name: name.to_string(),
            source_path: path.to_string(),
            content_hash: Some(hash.to_string()),
            scope_override: None,
            linked_metadata: None,
            installed_from_id: None,
        }
    }

    #[test]
    fn test_reconcile_inserts_new_resources() {
        let db = Database::new_in_memory().unwrap();
        let scanned = vec![
            make_scanned("skill-a", "/tmp/a", "hash-a"),
            make_scanned("skill-b", "/tmp/b", "hash-b"),
        ];
        let report = SyncEngine::reconcile(&db, &ResourceScope::Global, scanned).unwrap();
        assert_eq!(report.inserted, 2);
        assert_eq!(report.updated, 0);
        assert_eq!(report.deleted, 0);
        assert_eq!(db.count_resources_by_scope(&ResourceScope::Global).unwrap(), 2);
    }

    #[test]
    fn test_reconcile_updates_changed_resources() {
        let db = Database::new_in_memory().unwrap();
        // First sync
        let scanned1 = vec![make_scanned("skill-a", "/tmp/a", "hash-v1")];
        SyncEngine::reconcile(&db, &ResourceScope::Global, scanned1).unwrap();

        // Second sync with changed hash
        let scanned2 = vec![make_scanned("skill-a", "/tmp/a", "hash-v2")];
        let report = SyncEngine::reconcile(&db, &ResourceScope::Global, scanned2).unwrap();
        assert_eq!(report.inserted, 0);
        assert_eq!(report.updated, 1);
        assert_eq!(report.deleted, 0);

        let resource = db.get_resource_by_path("/tmp/a").unwrap().unwrap();
        assert_eq!(resource.content_hash.unwrap(), "hash-v2");
    }

    #[test]
    fn test_reconcile_deletes_missing_resources() {
        let db = Database::new_in_memory().unwrap();
        // First sync with 2 resources
        let scanned1 = vec![
            make_scanned("a", "/tmp/a", "h1"),
            make_scanned("b", "/tmp/b", "h2"),
        ];
        SyncEngine::reconcile(&db, &ResourceScope::Global, scanned1).unwrap();

        // Second sync with only 1 resource (b is gone)
        let scanned2 = vec![make_scanned("a", "/tmp/a", "h1")];
        let report = SyncEngine::reconcile(&db, &ResourceScope::Global, scanned2).unwrap();
        assert_eq!(report.inserted, 0);
        assert_eq!(report.updated, 0);
        assert_eq!(report.deleted, 1);
        assert_eq!(db.count_resources_by_scope(&ResourceScope::Global).unwrap(), 1);
    }

    #[test]
    fn test_reconcile_no_changes() {
        let db = Database::new_in_memory().unwrap();
        let scanned = vec![make_scanned("a", "/tmp/a", "h1")];
        SyncEngine::reconcile(&db, &ResourceScope::Global, scanned.clone()).unwrap();

        let report = SyncEngine::reconcile(&db, &ResourceScope::Global, scanned).unwrap();
        assert_eq!(report.inserted, 0);
        assert_eq!(report.updated, 0);
        assert_eq!(report.deleted, 0);
    }

    #[test]
    fn test_reconcile_mixed_operations() {
        let db = Database::new_in_memory().unwrap();
        let scanned1 = vec![
            make_scanned("a", "/tmp/a", "h1"),
            make_scanned("b", "/tmp/b", "h2"),
            make_scanned("c", "/tmp/c", "h3"),
        ];
        SyncEngine::reconcile(&db, &ResourceScope::Global, scanned1).unwrap();

        // a unchanged, b updated, c deleted, d new
        let scanned2 = vec![
            make_scanned("a", "/tmp/a", "h1"),
            make_scanned("b", "/tmp/b", "h2-new"),
            make_scanned("d", "/tmp/d", "h4"),
        ];
        let report = SyncEngine::reconcile(&db, &ResourceScope::Global, scanned2).unwrap();
        assert_eq!(report.inserted, 1);  // d
        assert_eq!(report.updated, 1);   // b
        assert_eq!(report.deleted, 1);   // c
        assert_eq!(db.count_resources_by_scope(&ResourceScope::Global).unwrap(), 3);
    }

    #[test]
    fn test_reconcile_different_scopes_independent() {
        let db = Database::new_in_memory().unwrap();
        let global = vec![make_scanned("g1", "/global/g1", "h1")];
        let library = vec![make_scanned("l1", "/lib/l1", "h2")];

        SyncEngine::reconcile(&db, &ResourceScope::Global, global).unwrap();
        SyncEngine::reconcile(&db, &ResourceScope::Library, library).unwrap();

        assert_eq!(db.count_resources_by_scope(&ResourceScope::Global).unwrap(), 1);
        assert_eq!(db.count_resources_by_scope(&ResourceScope::Library).unwrap(), 1);

        // Deleting global doesn't affect library
        let report = SyncEngine::reconcile(&db, &ResourceScope::Global, vec![]).unwrap();
        assert_eq!(report.deleted, 1);
        assert_eq!(db.count_resources_by_scope(&ResourceScope::Library).unwrap(), 1);
    }
}
