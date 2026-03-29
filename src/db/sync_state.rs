use rusqlite::params;

use crate::db::Database;
use crate::models::v2::SyncState;

impl Database {
    pub fn upsert_sync_state(&self, state: &SyncState) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sync_state (id, watched_path, last_hash, last_synced, status)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                state.id,
                state.watched_path,
                state.last_hash,
                state.last_synced,
                state.status,
            ],
        )?;
        Ok(())
    }

    pub fn get_sync_state(&self, id: &str) -> rusqlite::Result<Option<SyncState>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, watched_path, last_hash, last_synced, status
             FROM sync_state WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(SyncState {
                id: row.get(0)?,
                watched_path: row.get(1)?,
                last_hash: row.get(2)?,
                last_synced: row.get(3)?,
                status: row.get(4)?,
            })
        })?;
        match rows.next() {
            Some(result) => Ok(Some(result?)),
            None => Ok(None),
        }
    }

    pub fn get_sync_state_by_path(&self, watched_path: &str) -> rusqlite::Result<Option<SyncState>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, watched_path, last_hash, last_synced, status
             FROM sync_state WHERE watched_path = ?1",
        )?;
        let mut rows = stmt.query_map(params![watched_path], |row| {
            Ok(SyncState {
                id: row.get(0)?,
                watched_path: row.get(1)?,
                last_hash: row.get(2)?,
                last_synced: row.get(3)?,
                status: row.get(4)?,
            })
        })?;
        match rows.next() {
            Some(result) => Ok(Some(result?)),
            None => Ok(None),
        }
    }

    pub fn list_sync_states_by_status(&self, status: &str) -> rusqlite::Result<Vec<SyncState>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, watched_path, last_hash, last_synced, status
             FROM sync_state WHERE status = ?1",
        )?;
        let rows = stmt.query_map(params![status], |row| {
            Ok(SyncState {
                id: row.get(0)?,
                watched_path: row.get(1)?,
                last_hash: row.get(2)?,
                last_synced: row.get(3)?,
                status: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn delete_sync_state(&self, id: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM sync_state WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn delete_sync_states_by_path_prefix(&self, prefix: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("{}%", prefix);
        conn.execute(
            "DELETE FROM sync_state WHERE watched_path LIKE ?1",
            params![pattern],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::models::v2::SyncState;

    fn make_sync_state(id: &str, path: &str, status: &str) -> SyncState {
        SyncState {
            id: id.to_string(),
            watched_path: path.to_string(),
            last_hash: Some("hash123".to_string()),
            last_synced: Some("2026-03-01T00:00:00Z".to_string()),
            status: status.to_string(),
        }
    }

    #[test]
    fn test_upsert_and_get_sync_state() {
        let db = Database::new_in_memory().unwrap();
        let state = make_sync_state("s1", "/tmp/watch1", "active");

        db.upsert_sync_state(&state).unwrap();

        let fetched = db.get_sync_state("s1").unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, "s1");
        assert_eq!(fetched.watched_path, "/tmp/watch1");
        assert_eq!(fetched.last_hash, Some("hash123".to_string()));
        assert_eq!(fetched.last_synced, Some("2026-03-01T00:00:00Z".to_string()));
        assert_eq!(fetched.status, "active");

        // Non-existent returns None
        let missing = db.get_sync_state("nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_upsert_replaces_existing() {
        let db = Database::new_in_memory().unwrap();
        let state = make_sync_state("s1", "/tmp/watch1", "active");
        db.upsert_sync_state(&state).unwrap();

        // Upsert with same id but different values
        let updated = SyncState {
            id: "s1".to_string(),
            watched_path: "/tmp/watch1-updated".to_string(),
            last_hash: Some("newhash".to_string()),
            last_synced: Some("2026-03-02T00:00:00Z".to_string()),
            status: "stale".to_string(),
        };
        db.upsert_sync_state(&updated).unwrap();

        let fetched = db.get_sync_state("s1").unwrap().unwrap();
        assert_eq!(fetched.watched_path, "/tmp/watch1-updated");
        assert_eq!(fetched.last_hash, Some("newhash".to_string()));
        assert_eq!(fetched.last_synced, Some("2026-03-02T00:00:00Z".to_string()));
        assert_eq!(fetched.status, "stale");
    }

    #[test]
    fn test_get_sync_state_by_path() {
        let db = Database::new_in_memory().unwrap();
        let state = make_sync_state("s1", "/tmp/watch1", "active");
        db.upsert_sync_state(&state).unwrap();

        let fetched = db.get_sync_state_by_path("/tmp/watch1").unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, "s1");

        // Non-existent path returns None
        let missing = db.get_sync_state_by_path("/tmp/nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_list_sync_states_by_status() {
        let db = Database::new_in_memory().unwrap();
        let s1 = make_sync_state("s1", "/tmp/watch1", "active");
        let s2 = make_sync_state("s2", "/tmp/watch2", "active");
        let s3 = make_sync_state("s3", "/tmp/watch3", "stale");

        db.upsert_sync_state(&s1).unwrap();
        db.upsert_sync_state(&s2).unwrap();
        db.upsert_sync_state(&s3).unwrap();

        let active = db.list_sync_states_by_status("active").unwrap();
        assert_eq!(active.len(), 2);

        let stale = db.list_sync_states_by_status("stale").unwrap();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].id, "s3");

        let unknown = db.list_sync_states_by_status("unknown").unwrap();
        assert_eq!(unknown.len(), 0);
    }

    #[test]
    fn test_delete_sync_state() {
        let db = Database::new_in_memory().unwrap();
        let state = make_sync_state("s1", "/tmp/watch1", "active");
        db.upsert_sync_state(&state).unwrap();

        // Verify it exists
        assert!(db.get_sync_state("s1").unwrap().is_some());

        db.delete_sync_state("s1").unwrap();

        // Verify it's gone
        assert!(db.get_sync_state("s1").unwrap().is_none());

        // Deleting a non-existent id should not error
        db.delete_sync_state("nonexistent").unwrap();
    }

    #[test]
    fn test_delete_sync_states_by_path_prefix() {
        let db = Database::new_in_memory().unwrap();
        let s1 = make_sync_state("s1", "/home/user/project-a/file1", "active");
        let s2 = make_sync_state("s2", "/home/user/project-a/file2", "active");
        let s3 = make_sync_state("s3", "/home/user/project-b/file1", "active");

        db.upsert_sync_state(&s1).unwrap();
        db.upsert_sync_state(&s2).unwrap();
        db.upsert_sync_state(&s3).unwrap();

        // Delete all states under project-a
        db.delete_sync_states_by_path_prefix("/home/user/project-a/").unwrap();

        // project-a entries should be gone
        assert!(db.get_sync_state("s1").unwrap().is_none());
        assert!(db.get_sync_state("s2").unwrap().is_none());

        // project-b entry should still exist
        assert!(db.get_sync_state("s3").unwrap().is_some());
    }

    #[test]
    fn test_upsert_with_none_optional_fields() {
        let db = Database::new_in_memory().unwrap();
        let state = SyncState {
            id: "s1".to_string(),
            watched_path: "/tmp/watch1".to_string(),
            last_hash: None,
            last_synced: None,
            status: "pending".to_string(),
        };

        db.upsert_sync_state(&state).unwrap();

        let fetched = db.get_sync_state("s1").unwrap().unwrap();
        assert_eq!(fetched.last_hash, None);
        assert_eq!(fetched.last_synced, None);
        assert_eq!(fetched.status, "pending");
    }
}
