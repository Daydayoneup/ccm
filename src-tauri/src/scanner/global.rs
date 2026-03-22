use super::{scan_claude_dir, ScannedResource, compute_file_hash, v1_to_v2_resource_type};

/// Scan ~/.claude/ for global resources, returning v2 ScannedResource list
pub fn scan_global_resources() -> Vec<ScannedResource> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let claude_dir = home.join(".claude");
    if !claude_dir.is_dir() {
        return Vec::new();
    }
    let local_resources = scan_claude_dir(&claude_dir);
    local_resources
        .into_iter()
        .map(|lr| {
            let hash = compute_file_hash(&lr.path);
            ScannedResource {
                resource_type: v1_to_v2_resource_type(&lr.resource_type),
                name: lr.name,
                source_path: lr.path,
                content_hash: hash,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_scan_claude_dir_as_scanned_resources() {
        let tmp = TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");
        fs::create_dir_all(claude_dir.join("skills/my-skill")).unwrap();
        fs::write(
            claude_dir.join("skills/my-skill/SKILL.md"),
            "# Test Skill",
        )
        .unwrap();
        fs::create_dir_all(claude_dir.join("rules")).unwrap();
        fs::write(claude_dir.join("rules/style.md"), "# Style Rule").unwrap();

        let local = scan_claude_dir(&claude_dir);
        let resources: Vec<ScannedResource> = local
            .into_iter()
            .map(|lr| {
                let hash = compute_file_hash(&lr.path);
                ScannedResource {
                    resource_type: v1_to_v2_resource_type(&lr.resource_type),
                    name: lr.name,
                    source_path: lr.path,
                    content_hash: hash,
                }
            })
            .collect();

        assert_eq!(resources.len(), 2);
        assert!(resources
            .iter()
            .any(|r| r.name == "my-skill" && r.content_hash.is_some()));
        assert!(resources
            .iter()
            .any(|r| r.name == "style" && r.content_hash.is_some()));
    }
}
