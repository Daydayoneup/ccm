use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
}

/// Tauri command: Check if a path is a directory.
#[tauri::command]
pub fn path_is_directory(path: String) -> bool {
    Path::new(&path).is_dir()
}

/// Tauri command: Read file contents as a string.
/// If path is a directory containing SKILL.md (skill resource), reads that instead.
#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    let p = Path::new(&path);
    let actual_path = if p.is_dir() {
        let skill_md = p.join("SKILL.md");
        if skill_md.is_file() {
            skill_md
        } else {
            return Err(format!("Path is a directory: {}", path));
        }
    } else {
        p.to_path_buf()
    };
    fs::read_to_string(&actual_path)
        .map_err(|e| format!("Failed to read file {}: {}", actual_path.display(), e))
}

/// Tauri command: Write content to a file, creating parent directories if needed.
/// If path is a directory containing SKILL.md (skill resource), writes to that instead.
#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    let p = Path::new(&path);
    let actual_path = if p.is_dir() {
        let skill_md = p.join("SKILL.md");
        if skill_md.is_file() {
            skill_md
        } else {
            return Err(format!("Path is a directory: {}", path));
        }
    } else {
        p.to_path_buf()
    };
    if let Some(parent) = actual_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }
    fs::write(&actual_path, content)
        .map_err(|e| format!("Failed to write file {}: {}", actual_path.display(), e))
}

/// Tauri command: Delete a file or directory.
#[tauri::command]
pub fn delete_path(path: String) -> Result<(), String> {
    let p = Path::new(&path);

    if !p.exists() && p.symlink_metadata().is_err() {
        return Err(format!("Path does not exist: {}", path));
    }

    // Check if it's a symlink first
    if let Ok(meta) = p.symlink_metadata() {
        if meta.file_type().is_symlink() {
            return fs::remove_file(p)
                .map_err(|e| format!("Failed to remove symlink {}: {}", path, e));
        }
    }

    if p.is_dir() {
        fs::remove_dir_all(p).map_err(|e| format!("Failed to remove directory {}: {}", path, e))
    } else {
        fs::remove_file(p).map_err(|e| format!("Failed to remove file {}: {}", path, e))
    }
}

/// Tauri command: Create a directory (and all parent directories).
#[tauri::command]
pub fn create_directory(path: String) -> Result<(), String> {
    fs::create_dir_all(&path).map_err(|e| format!("Failed to create directory {}: {}", path, e))
}

/// Tauri command: List directory contents.
#[tauri::command]
pub fn list_directory(path: String) -> Result<Vec<FileEntry>, String> {
    let dir = Path::new(&path);
    if !dir.is_dir() {
        return Err(format!("Not a directory: {}", path));
    }

    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut result = Vec::new();
    for entry in entries.flatten() {
        let entry_path = entry.path();
        let name = entry
            .file_name()
            .to_string_lossy()
            .to_string();

        let (is_symlink, size, is_dir) = match entry_path.symlink_metadata() {
            Ok(meta) => {
                let is_sym = meta.file_type().is_symlink();
                let size = meta.len();
                // For is_dir, follow the symlink
                let is_dir = entry_path.is_dir();
                (is_sym, size, is_dir)
            }
            Err(_) => (false, 0, false),
        };

        result.push(FileEntry {
            name,
            path: entry_path.to_string_lossy().to_string(),
            is_dir,
            is_symlink,
            size,
        });
    }

    // Sort by name for consistent ordering
    result.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(result)
}

/// Tauri command: Rename/move a file or directory.
#[tauri::command]
pub fn rename_path(old_path: String, new_path: String) -> Result<(), String> {
    let old = Path::new(&old_path);
    let new = Path::new(&new_path);

    if !old.exists() {
        return Err(format!("Source path does not exist: {}", old_path));
    }
    if new.exists() {
        return Err(format!("Target path already exists: {}", new_path));
    }
    if let Some(parent) = new.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent dirs: {}", e))?;
    }
    fs::rename(old, new)
        .map_err(|e| format!("Failed to rename {} to {}: {}", old_path, new_path, e))
}

/// Tauri command: Compute SHA256 hash of a file.
#[tauri::command]
pub fn file_content_hash(path: String) -> Result<String, String> {
    let data = fs::read(&path).map_err(|e| format!("Failed to read file {}: {}", path, e))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_read_write_file() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.txt").to_string_lossy().to_string();

        write_file(file.clone(), "hello world".to_string()).unwrap();
        let content = read_file(file).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn test_write_file_creates_parents() {
        let tmp = TempDir::new().unwrap();
        let file = tmp
            .path()
            .join("a/b/c/test.txt")
            .to_string_lossy()
            .to_string();

        write_file(file.clone(), "nested".to_string()).unwrap();
        let content = read_file(file).unwrap();
        assert_eq!(content, "nested");
    }

    #[test]
    fn test_delete_file() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("to_delete.txt");
        fs::write(&file, "bye").unwrap();

        delete_path(file.to_string_lossy().to_string()).unwrap();
        assert!(!file.exists());
    }

    #[test]
    fn test_delete_directory() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("to_delete_dir");
        fs::create_dir_all(dir.join("subdir")).unwrap();
        fs::write(dir.join("subdir/file.txt"), "content").unwrap();

        delete_path(dir.to_string_lossy().to_string()).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn test_create_directory() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp
            .path()
            .join("new/nested/dir")
            .to_string_lossy()
            .to_string();

        create_directory(dir.clone()).unwrap();
        assert!(Path::new(&dir).is_dir());
    }

    #[test]
    fn test_list_directory() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("file_a.txt"), "a").unwrap();
        fs::write(tmp.path().join("file_b.txt"), "b").unwrap();
        fs::create_dir(tmp.path().join("subdir")).unwrap();

        let entries = list_directory(tmp.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(entries.len(), 3);

        // Sorted by name
        assert_eq!(entries[0].name, "file_a.txt");
        assert!(!entries[0].is_dir);
        assert_eq!(entries[1].name, "file_b.txt");
        assert!(!entries[1].is_dir);
        assert_eq!(entries[2].name, "subdir");
        assert!(entries[2].is_dir);
    }

    #[test]
    fn test_file_content_hash() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("hash_test.txt");
        fs::write(&file, "hello").unwrap();

        let hash = file_content_hash(file.to_string_lossy().to_string()).unwrap();
        // SHA256 of "hello"
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_file_content_hash_different_content() {
        let tmp = TempDir::new().unwrap();
        let file1 = tmp.path().join("a.txt");
        let file2 = tmp.path().join("b.txt");
        fs::write(&file1, "content1").unwrap();
        fs::write(&file2, "content2").unwrap();

        let hash1 = file_content_hash(file1.to_string_lossy().to_string()).unwrap();
        let hash2 = file_content_hash(file2.to_string_lossy().to_string()).unwrap();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_rename_path_file() {
        let tmp = TempDir::new().unwrap();
        let old = tmp.path().join("old.txt");
        let new_path = tmp.path().join("new.txt");
        fs::write(&old, "content").unwrap();

        rename_path(old.to_string_lossy().to_string(), new_path.to_string_lossy().to_string()).unwrap();
        assert!(!old.exists());
        assert!(new_path.exists());
        assert_eq!(fs::read_to_string(&new_path).unwrap(), "content");
    }

    #[test]
    fn test_rename_path_directory() {
        let tmp = TempDir::new().unwrap();
        let old_dir = tmp.path().join("old_dir");
        fs::create_dir(&old_dir).unwrap();
        fs::write(old_dir.join("file.txt"), "inside").unwrap();
        let new_dir = tmp.path().join("new_dir");

        rename_path(old_dir.to_string_lossy().to_string(), new_dir.to_string_lossy().to_string()).unwrap();
        assert!(!old_dir.exists());
        assert!(new_dir.join("file.txt").exists());
    }

    #[test]
    fn test_rename_path_target_exists() {
        let tmp = TempDir::new().unwrap();
        let old = tmp.path().join("old.txt");
        let existing = tmp.path().join("existing.txt");
        fs::write(&old, "a").unwrap();
        fs::write(&existing, "b").unwrap();

        let result = rename_path(old.to_string_lossy().to_string(), existing.to_string_lossy().to_string());
        assert!(result.is_err());
    }
}
