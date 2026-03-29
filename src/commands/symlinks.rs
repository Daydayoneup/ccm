use std::fs;
use std::path::Path;

/// Create a symlink from target pointing to source.
/// On Unix, this creates a symbolic link at `target` that points to `source`.
pub fn create_resource_link(source: &str, target: &str) -> Result<(), String> {
    let source_path = Path::new(source);
    let target_path = Path::new(target);

    if !source_path.exists() {
        return Err(format!("Source does not exist: {}", source));
    }

    if target_path.exists() || target_path.symlink_metadata().is_ok() {
        return Err(format!("Target already exists: {}", target));
    }

    // Ensure parent directory exists
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source_path, target_path)
            .map_err(|e| format!("Failed to create symlink: {}", e))?;
    }

    #[cfg(not(unix))]
    {
        return Err("Symlinks are only supported on Unix systems".to_string());
    }

    Ok(())
}

/// Remove a symlink at the given target path.
pub fn remove_resource_link(target: &str) -> Result<(), String> {
    let target_path = Path::new(target);

    if !target_path.symlink_metadata().is_ok() {
        return Err(format!("Target does not exist: {}", target));
    }

    let metadata = target_path
        .symlink_metadata()
        .map_err(|e| format!("Failed to read metadata: {}", e))?;

    if !metadata.file_type().is_symlink() {
        return Err(format!("Target is not a symlink: {}", target));
    }

    fs::remove_file(target_path).map_err(|e| format!("Failed to remove symlink: {}", e))?;

    Ok(())
}

/// Check if a symlink exists at the target and its source still exists.
pub fn check_symlink_valid(target: &str) -> bool {
    let target_path = Path::new(target);

    // Check if it's a symlink
    match target_path.symlink_metadata() {
        Ok(meta) => {
            if !meta.file_type().is_symlink() {
                return false;
            }
        }
        Err(_) => return false,
    }

    // Check if the symlink target actually exists
    target_path.exists()
}

/// Tauri command: Create a resource link (symlink).
#[tauri::command]
pub fn link_resource(source: String, target: String) -> Result<(), String> {
    create_resource_link(&source, &target)
}

/// Tauri command: Remove a resource link (symlink).
#[tauri::command]
pub fn unlink_resource(target: String) -> Result<(), String> {
    remove_resource_link(&target)
}

/// Tauri command: Check if a symlink is valid.
#[tauri::command]
pub fn is_symlink_valid(target: String) -> Result<bool, String> {
    Ok(check_symlink_valid(&target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_create_and_check_symlink() {
        let tmp = TempDir::new().unwrap();

        // Create a source file
        let source = tmp.path().join("source.txt");
        fs::write(&source, "hello").unwrap();

        // Create symlink
        let target = tmp.path().join("link.txt");
        create_resource_link(
            source.to_str().unwrap(),
            target.to_str().unwrap(),
        )
        .unwrap();

        // Verify symlink is valid
        assert!(check_symlink_valid(target.to_str().unwrap()));

        // Verify we can read through the symlink
        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn test_create_symlink_to_directory() {
        let tmp = TempDir::new().unwrap();

        let source_dir = tmp.path().join("source_dir");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("file.txt"), "content").unwrap();

        let target = tmp.path().join("link_dir");
        create_resource_link(
            source_dir.to_str().unwrap(),
            target.to_str().unwrap(),
        )
        .unwrap();

        assert!(check_symlink_valid(target.to_str().unwrap()));
        assert!(target.join("file.txt").exists());
    }

    #[test]
    fn test_remove_symlink() {
        let tmp = TempDir::new().unwrap();

        let source = tmp.path().join("source.txt");
        fs::write(&source, "hello").unwrap();

        let target = tmp.path().join("link.txt");
        create_resource_link(
            source.to_str().unwrap(),
            target.to_str().unwrap(),
        )
        .unwrap();

        // Remove the symlink
        remove_resource_link(target.to_str().unwrap()).unwrap();

        // Symlink should be gone
        assert!(!check_symlink_valid(target.to_str().unwrap()));
        // Source should still exist
        assert!(source.exists());
    }

    #[test]
    fn test_check_symlink_broken() {
        let tmp = TempDir::new().unwrap();

        let source = tmp.path().join("source.txt");
        fs::write(&source, "hello").unwrap();

        let target = tmp.path().join("link.txt");
        create_resource_link(
            source.to_str().unwrap(),
            target.to_str().unwrap(),
        )
        .unwrap();

        // Remove the source to make the symlink broken
        fs::remove_file(&source).unwrap();

        // Symlink exists but target doesn't
        assert!(!check_symlink_valid(target.to_str().unwrap()));
    }

    #[test]
    fn test_create_symlink_source_not_exists() {
        let tmp = TempDir::new().unwrap();
        let result = create_resource_link(
            tmp.path().join("nonexistent").to_str().unwrap(),
            tmp.path().join("link").to_str().unwrap(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_create_symlink_target_already_exists() {
        let tmp = TempDir::new().unwrap();

        let source = tmp.path().join("source.txt");
        fs::write(&source, "hello").unwrap();

        let target = tmp.path().join("target.txt");
        fs::write(&target, "existing").unwrap();

        let result = create_resource_link(
            source.to_str().unwrap(),
            target.to_str().unwrap(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_non_symlink_fails() {
        let tmp = TempDir::new().unwrap();

        let regular_file = tmp.path().join("regular.txt");
        fs::write(&regular_file, "content").unwrap();

        let result = remove_resource_link(regular_file.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_check_symlink_regular_file() {
        let tmp = TempDir::new().unwrap();
        let regular_file = tmp.path().join("regular.txt");
        fs::write(&regular_file, "content").unwrap();

        assert!(!check_symlink_valid(regular_file.to_str().unwrap()));
    }
}
