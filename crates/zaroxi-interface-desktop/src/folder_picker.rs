use std::path::PathBuf;

pub trait FolderPicker: Send + Sync {
    fn pick_folder(&self) -> Option<PathBuf>;
}

pub struct NativeFolderPicker;

impl FolderPicker for NativeFolderPicker {
    fn pick_folder(&self) -> Option<PathBuf> {
        log::info!("Explorer CTA: opening native folder picker dialog...");
        let result = rfd::FileDialog::new().pick_folder();
        match &result {
            Some(p) => log::info!("Explorer CTA: user selected folder {:?}", p),
            None => log::info!("Explorer CTA: user cancelled or picker unavailable"),
        }
        result
    }
}

pub struct FakeFolderPicker {
    result: Option<PathBuf>,
}

impl FakeFolderPicker {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self { result: path }
    }
}

impl FolderPicker for FakeFolderPicker {
    fn pick_folder(&self) -> Option<PathBuf> {
        self.result.clone()
    }
}

pub type DynFolderPicker = std::sync::Arc<dyn FolderPicker>;
