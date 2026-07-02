#![allow(dead_code)]
#![doc = "Filesystem-backed storage adapter implementing simple read/write via std::fs"]

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;

/// Simple filesystem storage adapter.
///
/// This adapter uses synchronous std::fs operations directly to avoid introducing
/// any dependency on core workspace/runtime crates. It provides straightforward
/// read/write helpers suitable for infrastructure wiring without pulling in
/// higher-layer transitive dependencies.
pub struct FileSystemStorage;

impl Default for FileSystemStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystemStorage {
    pub fn new() -> Self {
        FileSystemStorage
    }

    /// Read a UTF-8 file from disk.
    pub fn read_file(&self, path: &PathBuf) -> io::Result<String> {
        let mut s = String::new();
        let mut f = File::open(path)?;
        f.read_to_string(&mut s)?;
        Ok(s)
    }

    /// Write text content to disk at the given path, replacing/creating the file.
    pub fn write_file(&self, path: &PathBuf, contents: &str) -> io::Result<()> {
        let mut f = File::create(path)?;
        f.write_all(contents.as_bytes())?;
        Ok(())
    }
}

/// Convenience info mirroring the previous stub behaviour.
pub fn info() -> &'static str {
    env!("CARGO_PKG_NAME")
}
