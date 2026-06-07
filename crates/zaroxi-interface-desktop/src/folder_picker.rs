use std::path::PathBuf;

pub trait FolderPicker: Send + Sync {
    fn pick_folder(&self) -> Option<PathBuf>;
}

pub struct NativeFolderPicker;

impl FolderPicker for NativeFolderPicker {
    fn pick_folder(&self) -> Option<PathBuf> {
        if std::env::var("ZAROXI_DEBUG_CLICK").as_deref() == Ok("1") {
            eprintln!("ZAROXI_PICKER: NativeFolderPicker::pick_folder() called");
        }
        log::info!("Explorer CTA: opening native folder picker dialog...");
        let result = rfd::FileDialog::new().pick_folder();
        match &result {
            Some(p) => {
                log::info!("Explorer CTA: user selected folder {:?}", p);
                if std::env::var("ZAROXI_DEBUG_CLICK").as_deref() == Ok("1") {
                    eprintln!("ZAROXI_PICKER: pick_folder returned Some({})", p.display());
                }
            }
            None => {
                log::info!("Explorer CTA: user cancelled or picker unavailable");
                if std::env::var("ZAROXI_DEBUG_CLICK").as_deref() == Ok("1") {
                    eprintln!("ZAROXI_PICKER: pick_folder returned None");
                }
            }
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
