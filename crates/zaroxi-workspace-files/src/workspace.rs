use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path does not exist: {0}")]
    NotFound(String),
    #[error("Path is not a directory: {0}")]
    NotDirectory(String),
}

/// A directory entry for workspace operations.
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    /// Path to the entry.
    pub path: String,
    /// Name of the entry.
    pub name: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
}

pub struct WorkspaceLoader;

impl WorkspaceLoader {
    pub fn list_directory(path: &str) -> Result<Vec<DirectoryEntry>, WorkspaceError> {
        let dir_path = Path::new(path);
        if !dir_path.exists() {
            return Err(WorkspaceError::NotFound(path.to_string()));
        }
        if !dir_path.is_dir() {
            return Err(WorkspaceError::NotDirectory(path.to_string()));
        }

        let mut entries = Vec::new();
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

            let is_dir = path.is_dir();

            entries.push(DirectoryEntry { path: path.to_string_lossy().to_string(), name, is_dir });
        }

        Ok(entries)
    }

    pub fn create_file(path: &str) -> Result<(), WorkspaceError> {
        let file_path = Path::new(path);
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::File::create(file_path)?;
        Ok(())
    }

    pub fn create_directory(path: &str) -> Result<(), WorkspaceError> {
        let dir_path = Path::new(path);
        fs::create_dir_all(dir_path)?;
        Ok(())
    }

    pub fn rename_item(old_path: &str, new_path: &str) -> Result<(), WorkspaceError> {
        fs::rename(old_path, new_path)?;
        Ok(())
    }

    pub fn delete_item(path: &str) -> Result<(), WorkspaceError> {
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(WorkspaceError::NotFound(path.to_string()));
        }

        if path_obj.is_dir() {
            fs::remove_dir_all(path_obj)?;
        } else {
            fs::remove_file(path_obj)?;
        }
        Ok(())
    }

    pub fn save_file(path: &str, content: &str) -> Result<(), WorkspaceError> {
        let file_path = Path::new(path);
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(file_path, content)?;
        Ok(())
    }
}
