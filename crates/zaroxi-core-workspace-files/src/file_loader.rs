use memmap2::Mmap;
use std::fs;
use std::io::Read;
use thiserror::Error;

use crate::metadata::FileMetadata;

#[derive(Debug, Error)]
pub enum FileLoadError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("File too large: {0}")]
    TooLarge(String),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

/// Source of file content.
#[derive(Debug)]
pub enum FileSource {
    /// File loaded entirely into memory (small files).
    Memory(String),
    /// File memory‑mapped (large files).
    Mmap(Mmap),
}

impl FileSource {
    /// Get the content as a string slice.
    pub fn as_str(&self) -> &str {
        match self {
            FileSource::Memory(s) => s.as_str(),
            FileSource::Mmap(m) => {
                // SAFETY: mmap is valid for the lifetime of FileSource.
                unsafe { std::str::from_utf8_unchecked(m) }
            }
        }
    }

    /// Get the length in bytes.
    pub fn len(&self) -> usize {
        match self {
            FileSource::Memory(s) => s.len(),
            FileSource::Mmap(m) => m.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Strategy for loading a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileLoadStrategy {
    /// Load entire file into memory.
    Memory,
    /// Memory‑map the file.
    Mmap,
    /// Load only a preview (first N bytes).
    Preview(usize),
}

impl FileLoadStrategy {
    /// Determine the appropriate strategy based on file size.
    pub fn for_size(size: u64) -> Self {
        if size <= 10 * 1024 * 1024 {
            FileLoadStrategy::Memory
        } else if size <= 100 * 1024 * 1024 {
            FileLoadStrategy::Mmap
        } else {
            FileLoadStrategy::Mmap
        }
    }
}

pub struct FileLoader;

impl FileLoader {
    pub fn load_metadata(path: &str) -> Result<FileMetadata, FileLoadError> {
        let metadata = fs::metadata(path)?;
        Ok(FileMetadata::new(path.to_string(), metadata.len()))
    }

    /// Load a file using the appropriate strategy based on file size.
    pub fn load_file(path: &str) -> Result<(FileSource, u64), FileLoadError> {
        let metadata = fs::metadata(path)?;
        let size = metadata.len();
        let strategy = FileLoadStrategy::for_size(size);
        Self::load_file_with_strategy(path, strategy)
    }

    /// Load a file with a specific strategy.
    pub fn load_file_with_strategy(
        path: &str,
        strategy: FileLoadStrategy,
    ) -> Result<(FileSource, u64), FileLoadError> {
        let metadata = fs::metadata(path)?;
        let size = metadata.len();

        match strategy {
            FileLoadStrategy::Memory => {
                let content = fs::read_to_string(path)?;
                let source = FileSource::Memory(content);
                Ok((source, size))
            }
            FileLoadStrategy::Mmap => {
                let file = fs::File::open(path)?;
                let mmap = unsafe { Mmap::map(&file)? };
                let _ = std::str::from_utf8(&mmap)?;
                Ok((FileSource::Mmap(mmap), size))
            }
            FileLoadStrategy::Preview(max_bytes) => {
                let mut file = fs::File::open(path)?;
                let mut buffer = vec![0; max_bytes];
                let bytes_read = file.read(&mut buffer)?;
                let content = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                let source = FileSource::Memory(content);
                Ok((source, size))
            }
        }
    }
}
