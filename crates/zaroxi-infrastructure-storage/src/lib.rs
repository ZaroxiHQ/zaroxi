#![allow(dead_code)]
#![doc = "Filesystem-backed storage adapter implementing core-workspace-files::FileStorage"]

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use zaroxi_core_workspace_files::FileStorage;

/// Simple filesystem storage adapter.
///
/// This adapter implements the FileStorage port using synchronous std::fs
/// operations. It intentionally keeps semantics straightforward: read returns
/// the file contents as UTF-8 String, write replaces (creates/truncates)
/// the target file and writes bytes.
pub struct FileSystemStorage;

impl FileSystemStorage {
    pub fn new() -> Self {
        FileSystemStorage
    }
}

impl FileStorage for FileSystemStorage {
    fn read_file(&self, path: &PathBuf) -> io::Result<String> {
        let mut s = String::new();
        let mut f = File::open(path)?;
        f.read_to_string(&mut s)?;
        Ok(s)
    }

    fn write_file(&self, path: &PathBuf, contents: &str) -> io::Result<()> {
        let mut f = File::create(path)?;
        f.write_all(contents.as_bytes())?;
        Ok(())
    }
}

/// Convenience info mirroring the previous stub behaviour.
pub fn info() -> &'static str {
    env!("CARGO_PKG_NAME")
}
