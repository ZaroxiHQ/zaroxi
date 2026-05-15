/// Small types describing app shell layout regions.
///
/// These are logical descriptions only; rendering is the responsibility of the
/// runtime / UI crate. Keeping these types small makes testing and composition
/// easier later.
#[derive(Debug, Clone)]
pub enum Panel {
    Sidebar,
    Editor,
    StatusBar,
}
