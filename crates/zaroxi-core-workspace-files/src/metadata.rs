#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: String,
    pub size: u64,
}

impl FileMetadata {
    pub fn new(path: String, size: u64) -> Self {
        Self { path, size }
    }
}
