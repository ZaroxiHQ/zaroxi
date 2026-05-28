pub mod file_loader;
pub mod metadata;
pub mod workspace;

pub use file_loader::FileLoader;
pub use metadata::FileMetadata;
pub use workspace::WorkspaceLoader;

/// Port: FileStorage abstraction for reading/writing files on behalf of workspace.
use std::path::PathBuf;
use std::io;

/// Minimal filesystem-backed storage port used by application/workspace.
pub trait FileStorage: Send + Sync {
    /// Read a file from disk, returning its UTF-8 text content.
    fn read_file(&self, path: &PathBuf) -> io::Result<String>;

    /// Write text content to disk at the given path, replacing the file.
    fn write_file(&self, path: &PathBuf, contents: &str) -> io::Result<()>;
}

/// A simple disk-backed FileStorage implementation that delegates to the core IO crate.
pub struct DiskFileStorage;

impl DiskFileStorage {
    pub fn new() -> Self {
        DiskFileStorage
    }
}

impl FileStorage for DiskFileStorage {
    fn read_file(&self, path: &PathBuf) -> io::Result<String> {
        // Delegate to core io helpers.
        zaroxi_core_io::read_file(path.as_path())
    }

    fn write_file(&self, path: &PathBuf, contents: &str) -> io::Result<()> {
        zaroxi_core_io::write_file(path.as_path(), contents)
    }
}

/// Read direct children entries of a directory and return a vector of (path, is_dir).
///
/// This helper is intentionally minimal and synchronous — it mirrors the simple,
/// blocking style of the existing DiskFileStorage adapter and is suitable for
/// small workspaces and tests. Consumers (application/domain) may build the
/// richer tree model from this raw listing.
pub fn list_dir_entries(path: &PathBuf) -> io::Result<Vec<(PathBuf, bool)>> {
    let mut res: Vec<(PathBuf, bool)> = Vec::new();

    if !path.exists() {
        return Ok(res);
    }

    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let e = entry?;
            let p = e.path();
            let is_dir = p.is_dir();
            res.push((p, is_dir));
        }
    }

    Ok(res)
}
