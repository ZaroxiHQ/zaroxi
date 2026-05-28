#![allow(dead_code)]
#![doc = "Filesystem-backed storage adapter providing simple read/write helpers via core-io"]

use std::io;
use std::path::PathBuf;

use zaroxi_core_io;

/// Simple filesystem storage adapter implemented on top of the core IO helpers.
///
/// This adapter intentionally does not pull in the workspace-level types (which
/// may transitively depend on higher-level core crates). By depending only on
/// `zaroxi-core-io` we avoid creating a forbidden dependency edge to core-runtime.
pub struct FileSystemStorage;

impl FileSystemStorage {
    pub fn new() -> Self {
        FileSystemStorage
    }

    /// Read a UTF-8 file from disk.
    pub fn read_file(&self, path: &PathBuf) -> io::Result<String> {
        zaroxi_core_io::read_file(path.as_path())
    }

    /// Write text content to disk at the given path, replacing/creating the file.
    pub fn write_file(&self, path: &PathBuf, contents: &str) -> io::Result<()> {
        zaroxi_core_io::write_file(path.as_path(), contents)
    }
}

/// Convenience info mirroring the previous stub behaviour.
pub fn info() -> &'static str {
    env!("CARGO_PKG_NAME")
}
