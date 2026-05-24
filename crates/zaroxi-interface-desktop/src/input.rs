use zaroxi_application_workspace::editor_service::EditorService;
use crate::clipboard::InMemoryClipboard;

/// A tiny input bridge used in tests and harnesses to map high-level actions
/// into editor service calls. This intentionally does not pull platform event
/// types; it provides small helpers used by tests to simulate user actions.
pub struct InputBridge {
    pub svc: EditorService,
    pub clipboard: InMemoryClipboard,
}

impl InputBridge {
    pub fn new(svc: EditorService, clipboard: InMemoryClipboard) -> Self {
        Self { svc, clipboard }
    }

    /// Simulate typing characters (inserts or replaces current selection).
    pub fn type_text(&self, s: &str) {
        self.svc.type_text(s);
    }

    pub fn arrow_left(&self, shift: bool) {
        self.svc.arrow_left(shift);
    }
    pub fn arrow_right(&self, shift: bool) {
        self.svc.arrow_right(shift);
    }
    pub fn arrow_up(&self, shift: bool) {
        self.svc.arrow_up(shift);
    }
    pub fn arrow_down(&self, shift: bool) {
        self.svc.arrow_down(shift);
    }
    pub fn home(&self, shift: bool) {
        self.svc.home(shift);
    }
    pub fn end(&self, shift: bool) {
        self.svc.end(shift);
    }
    pub fn backspace(&self) {
        self.svc.backspace();
    }
    pub fn delete(&self) {
        self.svc.delete();
    }
    pub fn enter(&self) {
        self.svc.enter();
    }

    /// Copy: application returns the selection string which we place into the clipboard.
    pub fn copy(&self) {
        if let Some(t) = self.svc.copy_selection() {
            self.clipboard.set(t);
        }
    }

    /// Cut: copy then delete selection in application.
    pub fn cut(&self) {
        if let Some(t) = self.svc.copy_selection() {
            self.clipboard.set(t);
            self.svc.delete_selection();
        }
    }

    /// Paste: read clipboard and paste into application.
    pub fn paste(&self) {
        if let Some(t) = self.clipboard.get() {
            self.svc.paste_text(&t);
        }
    }

    /// Save current buffer to disk via application service.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        self.svc.save(path)
    }

    /// Reload current buffer from disk via application service.
    pub fn reload(&self, path: &std::path::Path) -> std::io::Result<()> {
        self.svc.reload(path)
    }

    /// Undo last edit (maps to Ctrl+Z semantics in tests/harnesses).
    pub fn undo(&self) {
        self.svc.undo();
    }

    /// Redo last undone edit (maps to Ctrl+Y / Ctrl+Shift+Z semantics).
    pub fn redo(&self) {
        self.svc.redo();
    }
}
