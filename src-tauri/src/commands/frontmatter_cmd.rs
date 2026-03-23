use tauri::State;
use crate::db::Database;
use crate::frontmatter::{self, SkillFrontmatter, SkillFrontmatterData};
use std::fs;

#[tauri::command]
pub fn parse_skill_frontmatter(file_path: String) -> Result<SkillFrontmatterData, String> {
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read {}: {}", file_path, e))?;
    frontmatter::parse_frontmatter(&content)
}

#[tauri::command]
pub fn save_skill_with_frontmatter(
    db: State<Database>,
    resource_id: String,
    file_path: String,
    frontmatter_data: SkillFrontmatter,
    body: String,
) -> Result<(), String> {
    let content = frontmatter::serialize_frontmatter(&frontmatter_data, &body)?;
    fs::write(&file_path, &content)
        .map_err(|e| format!("Failed to write {}: {}", file_path, e))?;

    // Mark resource as draft
    if let Some(mut resource) = db.get_resource(&resource_id)
        .map_err(|e| e.to_string())? {
        resource.is_draft = 1;
        resource.updated_at = chrono::Utc::now().to_rfc3339();
        if let Some(ref desc) = frontmatter_data.description {
            resource.description = Some(desc.clone());
        }
        db.update_resource(&resource).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn save_skill_raw_content(
    db: State<Database>,
    resource_id: String,
    file_path: String,
    content: String,
) -> Result<(), String> {
    // 1. Write file unconditionally (even if YAML is malformed)
    fs::write(&file_path, &content)
        .map_err(|e| format!("Failed to write {}: {}", file_path, e))?;

    // 2. Update DB (is_draft flag)
    if let Some(mut resource) = db.get_resource(&resource_id).map_err(|e| e.to_string())? {
        resource.is_draft = 1;
        resource.updated_at = chrono::Utc::now().to_rfc3339();

        // 3. Best-effort frontmatter parsing for description extraction
        if let Ok(parsed) = frontmatter::parse_frontmatter(&content) {
            if let Some(ref desc) = parsed.frontmatter.description {
                resource.description = Some(desc.clone());
            }
        }

        db.update_resource(&resource).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_resource(db: State<Database>, id: String) -> Result<Option<crate::models::v2::Resource>, String> {
    db.get_resource(&id).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_raw_content_roundtrip_with_valid_yaml() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("SKILL.md");
        let content = "---\nname: test-skill\ndescription: A test\n---\n\n# Hello\n";

        fs::write(&file_path, content).unwrap();
        let written = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written, content);

        let parsed = crate::frontmatter::parse_frontmatter(&written).unwrap();
        assert_eq!(parsed.frontmatter.name.as_deref(), Some("test-skill"));
        assert_eq!(parsed.frontmatter.description.as_deref(), Some("A test"));
    }

    #[test]
    fn test_raw_content_roundtrip_with_invalid_yaml() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("SKILL.md");
        let content = "---\nname: test\ninvalid yaml [[\n---\n\n# Body\n";

        fs::write(&file_path, content).unwrap();
        let written = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written, content);

        let result = crate::frontmatter::parse_frontmatter(&written);
        assert!(result.is_err());
    }
}
