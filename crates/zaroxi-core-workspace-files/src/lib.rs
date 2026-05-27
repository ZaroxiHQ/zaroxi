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
