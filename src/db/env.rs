use crate::db::Database;
use crate::models::v2::{EnvVar, MergedEnvVar};
use rusqlite::{params, Result};

impl Database {
    pub fn insert_env_var(&self, env_var: &EnvVar) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO claude_env_vars (id, project_id, key, value) VALUES (?1, ?2, ?3, ?4)",
            params![env_var.id, env_var.project_id, env_var.key, env_var.value],
        )?;
        Ok(())
    }

    pub fn list_env_vars(&self, project_id: Option<&str>) -> Result<Vec<EnvVar>> {
        let conn = self.conn.lock().unwrap();
        let mut vars = Vec::new();
        match project_id {
            None => {
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, key, value FROM claude_env_vars WHERE project_id IS NULL ORDER BY key",
                )?;
                let rows = stmt.query_map([], |row| {
                    Ok(EnvVar {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        key: row.get(2)?,
                        value: row.get(3)?,
                    })
                })?;
                for row in rows {
                    vars.push(row?);
                }
            }
            Some(pid) => {
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, key, value FROM claude_env_vars WHERE project_id = ?1 ORDER BY key",
                )?;
                let rows = stmt.query_map(params![pid], |row| {
                    Ok(EnvVar {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        key: row.get(2)?,
                        value: row.get(3)?,
                    })
                })?;
                for row in rows {
                    vars.push(row?);
                }
            }
        }
        Ok(vars)
    }

    pub fn delete_env_var(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM claude_env_vars WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn list_merged_env_vars(&self, project_id: &str) -> Result<Vec<MergedEnvVar>> {
        let conn = self.conn.lock().unwrap();
        // Fetch global vars
        let mut stmt = conn.prepare(
            "SELECT id, key, value FROM claude_env_vars WHERE project_id IS NULL ORDER BY key",
        )?;
        let global_rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut merged: std::collections::BTreeMap<String, MergedEnvVar> =
            std::collections::BTreeMap::new();
        for row in global_rows {
            let (id, key, value) = row?;
            merged.insert(
                key.clone(),
                MergedEnvVar {
                    id,
                    key,
                    value,
                    scope: "global".to_string(),
                },
            );
        }

        // Fetch project vars and override globals
        let mut stmt = conn.prepare(
            "SELECT id, key, value FROM claude_env_vars WHERE project_id = ?1 ORDER BY key",
        )?;
        let project_rows = stmt.query_map(params![project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for row in project_rows {
            let (id, key, value) = row?;
            merged.insert(
                key.clone(),
                MergedEnvVar {
                    id,
                    key,
                    value,
                    scope: "project".to_string(),
                },
            );
        }

        Ok(merged.into_values().collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::models::v2::{EnvVar, Project};

    fn make_project(id: &str) -> Project {
        Project {
            id: id.to_string(),
            name: format!("project-{}", id),
            path: format!("/home/user/projects/{}", id),
            language: Some("Rust".to_string()),
            last_scanned: None,
            pinned: 0,
            launch_count: 0,
        }
    }

    #[test]
    fn test_insert_and_list_global_env_vars() {
        let db = Database::new_in_memory().unwrap();
        let var = EnvVar {
            id: "g1".to_string(),
            project_id: None,
            key: "API_KEY".to_string(),
            value: "secret123".to_string(),
        };
        db.insert_env_var(&var).unwrap();

        let vars = db.list_env_vars(None).unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].id, "g1");
        assert_eq!(vars[0].key, "API_KEY");
        assert_eq!(vars[0].value, "secret123");
        assert!(vars[0].project_id.is_none());
    }

    #[test]
    fn test_insert_and_list_project_env_vars() {
        let db = Database::new_in_memory().unwrap();
        let project = make_project("p1");
        db.insert_project(&project).unwrap();

        let var = EnvVar {
            id: "pv1".to_string(),
            project_id: Some("p1".to_string()),
            key: "DB_URL".to_string(),
            value: "postgres://localhost".to_string(),
        };
        db.insert_env_var(&var).unwrap();

        let vars = db.list_env_vars(Some("p1")).unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].id, "pv1");
        assert_eq!(vars[0].key, "DB_URL");
        assert_eq!(vars[0].value, "postgres://localhost");
        assert_eq!(vars[0].project_id, Some("p1".to_string()));
    }

    #[test]
    fn test_upsert_env_var_same_scope_and_key() {
        let db = Database::new_in_memory().unwrap();
        let var = EnvVar {
            id: "g1".to_string(),
            project_id: None,
            key: "API_KEY".to_string(),
            value: "old_value".to_string(),
        };
        db.insert_env_var(&var).unwrap();

        let updated = EnvVar {
            id: "g1".to_string(),
            project_id: None,
            key: "API_KEY".to_string(),
            value: "new_value".to_string(),
        };
        db.insert_env_var(&updated).unwrap();

        let vars = db.list_env_vars(None).unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].value, "new_value");
    }

    #[test]
    fn test_delete_env_var() {
        let db = Database::new_in_memory().unwrap();
        let var = EnvVar {
            id: "g1".to_string(),
            project_id: None,
            key: "API_KEY".to_string(),
            value: "secret".to_string(),
        };
        db.insert_env_var(&var).unwrap();
        db.delete_env_var("g1").unwrap();

        let vars = db.list_env_vars(None).unwrap();
        assert!(vars.is_empty());
    }

    #[test]
    fn test_merged_env_vars_project_overrides_global() {
        let db = Database::new_in_memory().unwrap();
        let project = make_project("p1");
        db.insert_project(&project).unwrap();

        // Global vars
        let global_shared = EnvVar {
            id: "g1".to_string(),
            project_id: None,
            key: "SHARED_KEY".to_string(),
            value: "global_value".to_string(),
        };
        let global_only = EnvVar {
            id: "g2".to_string(),
            project_id: None,
            key: "GLOBAL_ONLY".to_string(),
            value: "only_global".to_string(),
        };
        db.insert_env_var(&global_shared).unwrap();
        db.insert_env_var(&global_only).unwrap();

        // Project var overrides SHARED_KEY
        let project_shared = EnvVar {
            id: "pv1".to_string(),
            project_id: Some("p1".to_string()),
            key: "SHARED_KEY".to_string(),
            value: "project_value".to_string(),
        };
        db.insert_env_var(&project_shared).unwrap();

        let merged = db.list_merged_env_vars("p1").unwrap();
        assert_eq!(merged.len(), 2);

        // Sorted by key: GLOBAL_ONLY first, then SHARED_KEY
        assert_eq!(merged[0].key, "GLOBAL_ONLY");
        assert_eq!(merged[0].value, "only_global");
        assert_eq!(merged[0].scope, "global");

        assert_eq!(merged[1].key, "SHARED_KEY");
        assert_eq!(merged[1].value, "project_value");
        assert_eq!(merged[1].scope, "project");
    }
}
