// zaroxi-core-io
// Auto-generated crate stub for the Zaroxi migration.
// Responsibility: Provide small blocking filesystem helpers used by higher-level workspace code.

#![allow(dead_code)]
#![allow(unused_imports)]

use std::io;
use std::path::Path;

/// Read a UTF-8 text file from disk.
pub fn read_file(path: &Path) -> io::Result<String> {
    std::fs::read_to_string(path)
}

/// Write text content to disk at the given path, replacing the file.
pub fn write_file(path: &Path, contents: &str) -> io::Result<()> {
    std::fs::write(path, contents)
}

/// Marker to make the crate non-empty for packaging.
pub fn _crate_marker() {
    // intentionally present
}
