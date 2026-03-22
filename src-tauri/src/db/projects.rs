use crate::db::Database;
use crate::models::v2::Project;
use rusqlite::{params, Result};

impl Database {
    pub fn insert_project(&self, project: &Project) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, path, language, last_scanned) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                project.id,
                project.name,
                project.path,
                project.language,
                project.last_scanned,
            ],
        )?;
        Ok(())
    }

    pub fn get_project(&self, id: &str) -> Result<Option<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, path, language, last_scanned, pinned, launch_count FROM projects WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                language: row.get(3)?,
                last_scanned: row.get(4)?,
                pinned: row.get(5)?,
                launch_count: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn get_project_by_path(&self, path: &str) -> Result<Option<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, path, language, last_scanned, pinned, launch_count FROM projects WHERE path = ?1",
        )?;
        let mut rows = stmt.query_map(params![path], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                language: row.get(3)?,
                last_scanned: row.get(4)?,
                pinned: row.get(5)?,
                launch_count: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, path, language, last_scanned, pinned, launch_count FROM projects ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                language: row.get(3)?,
                last_scanned: row.get(4)?,
                pinned: row.get(5)?,
                launch_count: row.get(6)?,
            })
        })?;
        let mut projects = Vec::new();
        for row in rows {
            projects.push(row?);
        }
        Ok(projects)
    }

    pub fn update_project(&self, project: &Project) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE projects SET name = ?1, path = ?2, language = ?3, last_scanned = ?4 WHERE id = ?5",
            params![
                project.name,
                project.path,
                project.language,
                project.last_scanned,
                project.id,
            ],
        )?;
        Ok(())
    }

    pub fn delete_project(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn count_projects(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
    }

    pub fn toggle_pin(&self, id: &str) -> Result<Option<Project>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE projects SET pinned = CASE WHEN pinned = 0 THEN 1 ELSE 0 END WHERE id = ?1",
            params![id],
        )?;
        drop(conn);
        self.get_project(id)
    }

    pub fn increment_launch_count(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE projects SET launch_count = launch_count + 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn list_projects_ranked(&self) -> Result<Vec<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, path, language, last_scanned, pinned, launch_count FROM projects ORDER BY pinned DESC, launch_count DESC, name ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                language: row.get(3)?,
                last_scanned: row.get(4)?,
                pinned: row.get(5)?,
                launch_count: row.get(6)?,
            })
        })?;
        let mut projects = Vec::new();
        for row in rows {
            projects.push(row?);
        }
        Ok(projects)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::models::v2::Project;

    fn make_project(id: &str) -> Project {
        Project {
            id: id.to_string(),
            name: format!("project-{}", id),
            path: format!("/home/user/projects/{}", id),
            language: Some("Rust".to_string()),
            last_scanned: Some("2026-03-01T00:00:00Z".to_string()),
            pinned: 0,
            launch_count: 0,
        }
    }

    #[test]
    fn test_insert_and_get_project() {
        let db = Database::new_in_memory().unwrap();
        let project = make_project("p1");

        db.insert_project(&project).unwrap();

        let fetched = db.get_project("p1").unwrap().expect("project should exist");
        assert_eq!(fetched.id, "p1");
        assert_eq!(fetched.name, "project-p1");
        assert_eq!(fetched.path, "/home/user/projects/p1");
        assert_eq!(fetched.language, Some("Rust".to_string()));
        assert_eq!(fetched.last_scanned, Some("2026-03-01T00:00:00Z".to_string()));
    }

    #[test]
    fn test_get_project_not_found() {
        let db = Database::new_in_memory().unwrap();
        let result = db.get_project("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_project_by_path() {
        let db = Database::new_in_memory().unwrap();
        let project = make_project("p1");
        db.insert_project(&project).unwrap();

        let fetched = db
            .get_project_by_path("/home/user/projects/p1")
            .unwrap()
            .expect("project should exist");
        assert_eq!(fetched.id, "p1");
        assert_eq!(fetched.path, "/home/user/projects/p1");
    }

    #[test]
    fn test_get_project_by_path_not_found() {
        let db = Database::new_in_memory().unwrap();
        let result = db.get_project_by_path("/no/such/path").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_projects_empty() {
        let db = Database::new_in_memory().unwrap();
        let projects = db.list_projects().unwrap();
        assert!(projects.is_empty());
    }

    #[test]
    fn test_list_projects_ordered_by_name() {
        let db = Database::new_in_memory().unwrap();

        // Insert in reverse alphabetical order
        let mut p_c = make_project("c");
        p_c.name = "charlie".to_string();
        let mut p_a = make_project("a");
        p_a.name = "alpha".to_string();
        let mut p_b = make_project("b");
        p_b.name = "bravo".to_string();

        db.insert_project(&p_c).unwrap();
        db.insert_project(&p_a).unwrap();
        db.insert_project(&p_b).unwrap();

        let projects = db.list_projects().unwrap();
        assert_eq!(projects.len(), 3);
        assert_eq!(projects[0].name, "alpha");
        assert_eq!(projects[1].name, "bravo");
        assert_eq!(projects[2].name, "charlie");
    }

    #[test]
    fn test_update_project() {
        let db = Database::new_in_memory().unwrap();
        let project = make_project("p1");
        db.insert_project(&project).unwrap();

        let mut updated = project.clone();
        updated.name = "updated-name".to_string();
        updated.language = Some("Go".to_string());
        updated.last_scanned = Some("2026-03-02T00:00:00Z".to_string());

        db.update_project(&updated).unwrap();

        let fetched = db.get_project("p1").unwrap().expect("project should exist");
        assert_eq!(fetched.name, "updated-name");
        assert_eq!(fetched.language, Some("Go".to_string()));
        assert_eq!(fetched.last_scanned, Some("2026-03-02T00:00:00Z".to_string()));
    }

    #[test]
    fn test_update_project_clear_optional_fields() {
        let db = Database::new_in_memory().unwrap();
        let project = make_project("p1");
        db.insert_project(&project).unwrap();

        let mut updated = project.clone();
        updated.language = None;
        updated.last_scanned = None;

        db.update_project(&updated).unwrap();

        let fetched = db.get_project("p1").unwrap().expect("project should exist");
        assert_eq!(fetched.language, None);
        assert_eq!(fetched.last_scanned, None);
    }

    #[test]
    fn test_delete_project() {
        let db = Database::new_in_memory().unwrap();
        let project = make_project("p1");
        db.insert_project(&project).unwrap();

        db.delete_project("p1").unwrap();

        let result = db.get_project("p1").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_project_nonexistent() {
        let db = Database::new_in_memory().unwrap();
        // Deleting a nonexistent project should not error
        db.delete_project("nonexistent").unwrap();
    }

    #[test]
    fn test_count_projects() {
        let db = Database::new_in_memory().unwrap();
        assert_eq!(db.count_projects().unwrap(), 0);

        db.insert_project(&make_project("p1")).unwrap();
        assert_eq!(db.count_projects().unwrap(), 1);

        db.insert_project(&make_project("p2")).unwrap();
        assert_eq!(db.count_projects().unwrap(), 2);

        db.delete_project("p1").unwrap();
        assert_eq!(db.count_projects().unwrap(), 1);
    }

    #[test]
    fn test_insert_duplicate_id_fails() {
        let db = Database::new_in_memory().unwrap();
        let project = make_project("p1");
        db.insert_project(&project).unwrap();

        let result = db.insert_project(&project);
        assert!(result.is_err());
    }

    #[test]
    fn test_insert_duplicate_path_fails() {
        let db = Database::new_in_memory().unwrap();
        let p1 = make_project("p1");
        db.insert_project(&p1).unwrap();

        // Different id but same path
        let mut p2 = make_project("p2");
        p2.path = p1.path.clone();

        let result = db.insert_project(&p2);
        assert!(result.is_err());
    }

    #[test]
    fn test_toggle_pin() {
        let db = Database::new_in_memory().unwrap();
        let project = make_project("p1");
        db.insert_project(&project).unwrap();
        let p = db.get_project("p1").unwrap().unwrap();
        assert_eq!(p.pinned, 0);
        let p = db.toggle_pin("p1").unwrap().unwrap();
        assert_eq!(p.pinned, 1);
        let p = db.toggle_pin("p1").unwrap().unwrap();
        assert_eq!(p.pinned, 0);
    }

    #[test]
    fn test_toggle_pin_nonexistent() {
        let db = Database::new_in_memory().unwrap();
        let result = db.toggle_pin("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_increment_launch_count() {
        let db = Database::new_in_memory().unwrap();
        db.insert_project(&make_project("p1")).unwrap();
        db.increment_launch_count("p1").unwrap();
        let p = db.get_project("p1").unwrap().unwrap();
        assert_eq!(p.launch_count, 1);
        db.increment_launch_count("p1").unwrap();
        let p = db.get_project("p1").unwrap().unwrap();
        assert_eq!(p.launch_count, 2);
    }

    #[test]
    fn test_list_projects_ranked() {
        let db = Database::new_in_memory().unwrap();
        let mut p_a = make_project("a"); p_a.name = "alpha".to_string();
        let mut p_b = make_project("b"); p_b.name = "bravo".to_string();
        let mut p_c = make_project("c"); p_c.name = "charlie".to_string();
        db.insert_project(&p_a).unwrap();
        db.insert_project(&p_b).unwrap();
        db.insert_project(&p_c).unwrap();
        db.toggle_pin("b").unwrap();
        db.increment_launch_count("c").unwrap();
        db.increment_launch_count("c").unwrap();
        db.increment_launch_count("a").unwrap();
        let ranked = db.list_projects_ranked().unwrap();
        assert_eq!(ranked[0].name, "bravo");   // pinned
        assert_eq!(ranked[1].name, "charlie"); // 2 launches
        assert_eq!(ranked[2].name, "alpha");   // 1 launch
    }
}
