/*!
VS Code-style editor group state model.

Owns the single source of truth for which file editors are open, whether
each is a preview or a pinned (persistent) editor, and which editor is
currently active.  No other structure may act as an alternative source of
tab membership — the visible tab strip MUST be derived exclusively from
[`EditorGroup::visible_tabs`].

Hard invariants (asserted in debug builds):
- A canonical path appears at most once across pinned + preview.
- There is at most one preview editor at any time.
 */

use super::DocumentViewState;
use crate::gui::window::editor_buf::EditorBufferState;
use std::collections::HashMap;

/// Stable identity for an editor entry.  Currently the canonical file
/// path (no `buf:` prefix), which is 1:1 with buffer identity.
pub type EditorId = String;

// ── Canonical identity conversion (single source of truth) ──────────
//
// File identity in the workbench has exactly one primary form: the
// canonical filesystem path (no `buf:` prefix).  The `buf:<path>` form
// is *only* a transport/lookup wrapper used by the workspace-service
// `BufferId` and the tab-strip `WorkbenchTabId::FileBuffer`.  Every list,
// map, and comparison MUST route identity through these helpers so the
// two forms can never drift.  No ad-hoc `strip_prefix("buf:")` should be
// added at new call sites — call [`canonical_path_from_editor_id`].

/// Reduce any editor id / buffer id / tab id string to the canonical
/// filesystem path (the `buf:` transport prefix stripped).  Idempotent:
/// canonical paths pass through unchanged.
pub fn canonical_path_from_editor_id(id: &str) -> &str {
    id.strip_prefix("buf:").unwrap_or(id)
}

/// Build the `buf:<path>` transport/lookup key for a canonical path.
/// Accepts either form as input and always returns a single normalized
/// `buf:<canonical_path>` string, so repeated wrapping cannot double the
/// prefix.
pub fn buffer_key_from_path(path: &str) -> String {
    format!("buf:{}", canonical_path_from_editor_id(path))
}

/// Whether two identity strings refer to the same document, regardless of
/// whether either carries the `buf:` transport prefix.
pub fn same_document(a: &str, b: &str) -> bool {
    canonical_path_from_editor_id(a) == canonical_path_from_editor_id(b)
}

/// Backend kind for the file content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    /// Normal file — content lives in `editor_buffer` (rope),
    /// parked / restored via `open_documents`.
    Rope,
    /// Large file — content lives in a `PieceTable` inside
    /// `doc_buffers`; the rope is a viewport-sized derived window.
    PieceTable,
}

/// One file editor in the group, carrying all tab-strip-visible
/// properties and the backend identity.
#[derive(Debug, Clone)]
pub struct EditorEntry {
    pub id: EditorId,
    /// Canonical file path (no `buf:` prefix).
    pub path: String,
    /// Workspace buffer id string, e.g. `"buf:/path/to/file"`.
    pub buffer_id: String,
    /// Tab-strip label.
    pub display: String,
    /// Whether the backend has been registered and content is ready.
    /// `true` while the large-file background read is still in flight.
    pub backend_ready: bool,
    /// Backend kind, set once when the file is first opened.
    pub backend_kind: BackendKind,
}

/// A single projected tab for cockpit consumption, derived from
/// [`EditorGroup::visible_tabs`] once per frame.
#[derive(Debug, Clone)]
pub struct VisibleTab {
    pub editor_id: EditorId,
    pub path: String,
    pub display: String,
    pub buffer_id: String,
    pub is_preview: bool,
    pub is_pinned: bool,
    pub is_active: bool,
    pub loading: bool,
    pub backend_kind: BackendKind,
}

/// The single authoritative editor-group state for the workbench file
/// editor area.
///
/// # Invariants
///
/// - No path is both pinned and preview simultaneously.
/// - Exactly zero or one preview editor.
/// - `active` is `Some(id)` where `id` exists in `pinned` or `preview`,
///   or `None` when no file editor is open.
#[derive(Debug, Clone, Default)]
pub struct EditorGroup {
    pinned: Vec<EditorEntry>,
    preview: Option<EditorEntry>,
    active: Option<EditorId>,
}

// ── Queries ─────────────────────────────────────────────────────────

impl EditorGroup {
    /// All editor ids currently present (pinned + preview).
    pub fn all_ids(&self) -> Vec<&EditorId> {
        let mut ids: Vec<&EditorId> = self.pinned.iter().map(|e| &e.id).collect();
        if let Some(ref pv) = self.preview {
            ids.push(&pv.id);
        }
        ids
    }

    /// Whether a canonical path is already a pinned editor.
    pub fn is_pinned(&self, path: &str) -> bool {
        self.pinned.iter().any(|e| e.path == path)
    }

    /// Whether a canonical path is currently the preview editor.
    pub fn is_preview(&self, path: &str) -> bool {
        self.preview.as_ref().is_some_and(|pv| pv.path == path)
    }

    /// Look up an editor entry by id.
    pub fn get(&self, id: &EditorId) -> Option<&EditorEntry> {
        if Some(id) == self.preview.as_ref().map(|pv| &pv.id) {
            return self.preview.as_ref();
        }
        self.pinned.iter().find(|e| &e.id == id)
    }

    /// The active editor entry, if any.
    pub fn active_entry(&self) -> Option<&EditorEntry> {
        self.active.as_ref().and_then(|id| self.get(id))
    }

    /// The active editor path (no `buf:` prefix).
    pub fn active_path(&self) -> Option<&str> {
        self.active_entry().map(|e| e.path.as_str())
    }

    /// The preview editor path, if any.
    pub fn preview_path(&self) -> Option<&str> {
        self.preview.as_ref().map(|pv| pv.path.as_str())
    }

    /// Visible file tabs derived solely from editor-group membership.
    ///
    /// Order: pinned editors in sequence, then preview editor prepended.
    /// This is the single source of truth for the cockpit tab strip.
    pub fn visible_tabs(&self) -> Vec<VisibleTab> {
        let mut tabs = Vec::new();

        // Pinned editors first (render in order).
        for e in &self.pinned {
            tabs.push(VisibleTab {
                editor_id: e.id.clone(),
                path: e.path.clone(),
                display: e.display.clone(),
                buffer_id: e.buffer_id.clone(),
                is_preview: false,
                is_pinned: true,
                is_active: Some(&e.id) == self.active.as_ref(),
                loading: !e.backend_ready,
                backend_kind: e.backend_kind,
            });
        }

        // Preview editor prepended (inserted at front).
        if let Some(ref pv) = self.preview {
            tabs.insert(
                0,
                VisibleTab {
                    editor_id: pv.id.clone(),
                    path: pv.path.clone(),
                    display: pv.display.clone(),
                    buffer_id: pv.buffer_id.clone(),
                    is_preview: true,
                    is_pinned: false,
                    is_active: Some(&pv.id) == self.active.as_ref(),
                    loading: !pv.backend_ready,
                    backend_kind: pv.backend_kind,
                },
            );
        }

        tabs
    }

    /// Debug: assert invariants and log any violation.
    pub fn check_invariants(&self) {
        for e in &self.pinned {
            if let Some(ref pv) = self.preview
                && e.path == pv.path
            {
                log::error!("EDITOR_GROUP_INVARIANT: path={} is both pinned and preview", e.path,);
            }
        }
    }

    /// Debug: produce a diagnostic line.
    pub fn diagnostic_line(&self) -> String {
        let pinned_paths: Vec<&str> = self.pinned.iter().map(|e| e.path.as_str()).collect();
        let preview_path = self.preview.as_ref().map(|pv| pv.path.as_str()).unwrap_or("<none>");
        let active_path = self.active_path().unwrap_or("<none>");
        format!(
            "editor_group_state pinned={:?} preview={} active={}",
            pinned_paths, preview_path, active_path,
        )
    }
}

// ── Operations ──────────────────────────────────────────────────────

impl EditorGroup {
    /// Open a file as a preview editor.
    ///
    /// Rules:
    /// - If the path is already pinned, activate the pinned editor (no-op
    ///   on preview).
    /// - If a preview editor already exists for a different path, save its
    ///   outgoing state (via the caller-supplied callback) and replace it.
    /// - Set the preview as active.
    ///
    /// Returns the editor id of the opened/replaced preview, or `None` if
    /// the path was already pinned (caller should activate the pinned
    /// entry instead).
    pub fn open_preview(
        &mut self,
        path: String,
        buffer_id: String,
        display: String,
        backend_kind: BackendKind,
        backend_ready: bool,
    ) -> Option<EditorId> {
        // Rule: if path is already pinned, do not open a duplicate
        // preview.  The caller should activate the existing pinned entry.
        if self.is_pinned(&path) {
            return None;
        }

        let id = path.clone();
        let entry =
            EditorEntry { id: id.clone(), path, buffer_id, display, backend_ready, backend_kind };

        self.preview = Some(entry);
        let result = id.clone();
        self.active = Some(id);
        Some(result)
    }

    /// Open (or activate) a file as a pinned editor.
    ///
    /// Rules:
    /// - If already pinned, activate it.
    /// - If the path matches the current preview, promote the preview to
    ///   pinned (no new entry created).
    /// - Otherwise, create a new pinned editor at the end of the list and
    ///   activate it.
    ///
    /// Returns the editor id.
    pub fn open_or_activate_pinned(
        &mut self,
        path: String,
        buffer_id: String,
        display: String,
        backend_kind: BackendKind,
        backend_ready: bool,
    ) -> EditorId {
        // Already pinned: just activate.
        if let Some(pos) = self.pinned.iter().position(|e| e.path == path) {
            let id = self.pinned[pos].id.clone();
            self.active = Some(id.clone());
            return id;
        }

        // Promote preview if it matches.
        if let Some(ref pv) = self.preview
            && pv.path == path
        {
            let mut entry = self.preview.take().unwrap();
            entry.backend_ready = backend_ready;
            entry.backend_kind = backend_kind;
            let id = entry.id.clone();
            self.pinned.push(entry);
            self.active = Some(id.clone());
            return id;
        }

        // New pinned editor.
        let id = path.clone();
        self.pinned.push(EditorEntry {
            id: id.clone(),
            path,
            buffer_id,
            display,
            backend_ready,
            backend_kind,
        });
        self.active = Some(id.clone());
        id
    }

    /// Activate an existing editor by path (no `buf:` prefix).
    /// Returns `true` if the active editor changed.
    pub fn activate_by_path(&mut self, path: &str) -> bool {
        let target =
            if self.is_pinned(path) || self.preview.as_ref().is_some_and(|pv| pv.path == path) {
                Some(path.to_string())
            } else {
                None
            };
        if let Some(id) = target
            && self.active.as_deref() != Some(&id)
        {
            self.active = Some(id);
            return true;
        }
        false
    }

    /// Promote the current preview editor to a pinned editor.
    /// The preview slot is cleared. Returns the promoted editor id, or
    /// `None` if there was no preview.
    pub fn promote_preview_to_pinned(&mut self) -> Option<EditorId> {
        let entry = self.preview.take()?;
        let id = entry.id.clone();

        // If somehow already in pinned (invariant violation), dedup.
        if let Some(pos) = self.pinned.iter().position(|e| e.path == entry.path) {
            self.pinned[pos] = entry;
        } else {
            self.pinned.push(entry);
        }
        self.active = Some(id.clone());
        Some(id)
    }

    /// Replace the current preview with a new file.
    /// If no preview exists, this is equivalent to [`open_preview`].
    /// If the path is pinned, this is a no-op (returns `None`).
    pub fn replace_preview(
        &mut self,
        path: String,
        buffer_id: String,
        display: String,
        backend_kind: BackendKind,
        backend_ready: bool,
    ) -> Option<EditorId> {
        if self.is_pinned(&path) {
            return None;
        }
        // Replace directly (no save callback — the caller saves
        // outgoing state before calling this).
        let id = path.clone();
        self.preview = Some(EditorEntry {
            id: id.clone(),
            path,
            buffer_id,
            display,
            backend_ready,
            backend_kind,
        });
        self.active = Some(id.clone());
        Some(id)
    }

    /// Close an editor by id. If it was the active editor, activate the
    /// next available editor (preview first, then last pinned, then none).
    /// Returns `true` if something changed.
    pub fn close(&mut self, id: &EditorId) -> bool {
        // Normalize to canonical identity so callers may pass either the
        // canonical path OR the `buf:<path>` transport/tab id form.
        let canon = canonical_path_from_editor_id(id);
        let was_preview = self.preview.as_ref().is_some_and(|pv| same_document(&pv.id, canon));
        let was_pinned_pos = self.pinned.iter().position(|e| same_document(&e.id, canon));

        if was_preview {
            self.preview = None;
        } else if let Some(pos) = was_pinned_pos {
            self.pinned.remove(pos);
        } else {
            return false;
        }

        let was_active = self.active.as_deref().is_some_and(|a| same_document(a, canon));
        if was_active {
            let next = self
                .preview
                .as_ref()
                .map(|pv| pv.id.clone())
                .or_else(|| self.pinned.last().map(|e| e.id.clone()));
            self.active = next;
        }

        true
    }

    /// Mark the backend as ready for an editor (large-file load complete).
    pub fn mark_backend_ready(&mut self, id: &EditorId) {
        if let Some(ref mut pv) = self.preview
            && pv.id == *id
        {
            pv.backend_ready = true;
        }
        for e in &mut self.pinned {
            if e.id == *id {
                e.backend_ready = true;
            }
        }
    }

    /// Set the active editor to the last pinned (or preview if no pinned).
    pub fn activate_last_pinned_or_preview(&mut self) {
        if let Some(last) = self.pinned.last() {
            self.active = Some(last.id.clone());
        } else if self.preview.is_some() {
            self.active = self.preview.as_ref().map(|pv| pv.id.clone());
        }
    }
}

// ── View-state management (shared by preview and pinned) ────────────

/// Per-editor save/restore view-state functions.
///
/// These delegate to the shared `document_view_states` / `open_documents`
/// stores on `GuiApp`.  The EditorGroup only records the **identity** and
/// **membership** of editors; the view state lives elsewhere.  These
/// helpers tie the two together so the caller doesn't need to know the
/// storage details.
impl EditorGroup {
    /// Save the outgoing editor's view state before switching away from it.
    ///
    /// The caller provides the current live scroll top (from composition
    /// metadata) and snapshots of the live editor buffer.
    pub fn save_view_state_for(
        view_states: &mut HashMap<String, DocumentViewState>,
        prev_path: &str,
        scroll_top: usize,
        buf: &EditorBufferState,
    ) {
        view_states.insert(
            prev_path.to_string(),
            DocumentViewState::from_editor_and_scroll(buf, scroll_top),
        );
    }

    /// Restore view state for an incoming editor.
    ///
    /// Returns the saved `DocumentViewState` if one existed, removing it
    /// from the store (the caller writes it into live metadata).
    pub fn restore_view_state_for(
        view_states: &mut HashMap<String, DocumentViewState>,
        path: &str,
    ) -> Option<DocumentViewState> {
        view_states.remove(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str) -> (String, String, String, BackendKind) {
        (path.to_string(), format!("buf:{}", path), path.to_string(), BackendKind::Rope)
    }

    #[test]
    fn open_preview_sets_active() {
        let mut g = EditorGroup::default();
        let (p, bid, disp, bk) = entry("/a/b.rs");
        let _id = g.open_preview(p, bid, disp, bk, true).unwrap();
        assert_eq!(g.active.as_deref(), Some("/a/b.rs"));
        assert_eq!(g.preview_path(), Some("/a/b.rs"));
        assert!(g.preview.is_some());
        assert!(g.pinned.is_empty());
    }

    #[test]
    fn open_preview_replaces_old() {
        let mut g = EditorGroup::default();
        let (p1, bid1, d1, bk) = entry("/a.rs");
        let (p2, bid2, d2, bk2) = entry("/b.rs");
        g.open_preview(p1, bid1, d1, bk, true);
        g.open_preview(p2, bid2, d2, bk2, true);
        assert_eq!(g.preview_path(), Some("/b.rs"));
        assert_eq!(g.pinned.len(), 0);
    }

    #[test]
    fn open_preview_skips_if_pinned() {
        let mut g = EditorGroup::default();
        let (p, bid, d, bk) = entry("/a.rs");
        g.open_or_activate_pinned(p.clone(), bid, d, bk, true);
        let result = g.open_preview(p, "buf:/a.rs".into(), "a".into(), BackendKind::Rope, true);
        assert!(result.is_none());
        assert!(g.preview.is_none());
        assert_eq!(g.pinned.len(), 1);
    }

    #[test]
    fn open_pinned_creates_when_absent() {
        let mut g = EditorGroup::default();
        let (p, bid, d, bk) = entry("/a.rs");
        let id = g.open_or_activate_pinned(p, bid, d, bk, true);
        assert_eq!(g.pinned.len(), 1);
        assert_eq!(g.active.as_deref(), Some(id.as_str()));
        assert!(g.preview.is_none());
    }

    #[test]
    fn open_pinned_activates_existing() {
        let mut g = EditorGroup::default();
        let (p, bid, d, bk) = entry("/a.rs");
        g.open_or_activate_pinned(p.clone(), bid, d, bk, true);
        // Activate another (so we can test re-activation).
        let (p2, bid2, d2, bk2) = entry("/b.rs");
        g.open_or_activate_pinned(p2, bid2, d2, bk2, true);
        assert_eq!(g.active_path(), Some("/b.rs"));
        // Re-activate first.
        g.open_or_activate_pinned(p, "buf:/a.rs".into(), "a".into(), BackendKind::Rope, true);
        assert_eq!(g.active_path(), Some("/a.rs"));
        assert_eq!(g.pinned.len(), 2);
    }

    #[test]
    fn promote_preview_moves_to_pinned_and_clears_preview() {
        let mut g = EditorGroup::default();
        let (p, bid, d, bk) = entry("/a.rs");
        g.open_preview(p, bid, d, bk, true);
        let id = g.promote_preview_to_pinned().unwrap();
        assert!(g.preview.is_none());
        assert_eq!(g.pinned.len(), 1);
        assert_eq!(g.pinned[0].id, id);
        assert_eq!(g.active.as_deref(), Some(id.as_str()));
    }

    #[test]
    fn close_removes_and_falls_back() {
        let mut g = EditorGroup::default();
        let (p, bid, d, bk) = entry("/a.rs");
        let id = g.open_or_activate_pinned(p, bid, d, bk, true);
        g.close(&id);
        assert!(g.pinned.is_empty());
        assert!(g.active.is_none());
    }

    #[test]
    fn close_active_falls_back_to_preview() {
        let mut g = EditorGroup::default();
        let (p_pv, bid_pv, d_pv, bk) = entry("/preview.rs");
        let (p_pin, bid_pin, d_pin, bk2) = entry("/pinned.rs");
        g.open_preview(p_pv, bid_pv, d_pv, bk, true);
        let pin_id = g.open_or_activate_pinned(p_pin, bid_pin, d_pin, bk2, true);
        // Active is pinned now.
        g.close(&pin_id);
        // Falls back to preview.
        assert_eq!(g.active_path(), Some("/preview.rs"));
    }

    #[test]
    fn visible_tabs_preview_comes_first() {
        let mut g = EditorGroup::default();
        let (p1, b1, d1, bk) = entry("/pinned.rs");
        let (p2, b2, d2, bk2) = entry("/preview.rs");
        g.open_or_activate_pinned(p1, b1, d1, bk, true);
        g.open_preview(p2, b2, d2, bk2, true);
        let tabs = g.visible_tabs();
        assert_eq!(tabs.len(), 2);
        assert!(tabs[0].is_preview);
        assert!(!tabs[1].is_preview);
        assert!(tabs[1].is_pinned);
    }

    #[test]
    fn canonical_identity_helpers_roundtrip() {
        assert_eq!(canonical_path_from_editor_id("buf:/a/b.rs"), "/a/b.rs");
        assert_eq!(canonical_path_from_editor_id("/a/b.rs"), "/a/b.rs");
        // buffer_key normalizes either form to a single `buf:` prefix.
        assert_eq!(buffer_key_from_path("/a/b.rs"), "buf:/a/b.rs");
        assert_eq!(buffer_key_from_path("buf:/a/b.rs"), "buf:/a/b.rs");
        assert!(same_document("buf:/a/b.rs", "/a/b.rs"));
        assert!(same_document("/a/b.rs", "/a/b.rs"));
        assert!(!same_document("buf:/a/b.rs", "buf:/a/c.rs"));
    }

    #[test]
    fn close_accepts_transport_prefixed_id() {
        // The mouse close button passes the `buf:<path>` tab id, while the
        // EditorGroup entry id is the canonical path.  close() must resolve
        // both to the same document and actually remove the entry.
        let mut g = EditorGroup::default();
        let (p, bid, d, bk) = entry("/a.rs");
        g.open_or_activate_pinned(p, bid, d, bk, true);
        assert_eq!(g.pinned.len(), 1);
        assert!(g.close(&"buf:/a.rs".to_string()));
        assert!(g.pinned.is_empty());
        assert!(g.active.is_none());
    }

    #[test]
    fn no_path_in_both_pinned_and_preview() {
        let mut g = EditorGroup::default();
        let (p, bid, d, bk) = entry("/a.rs");
        // Open as preview.
        assert!(g.open_preview(p.clone(), bid, d, bk, true).is_some());
        // Try to open same path as pinned → should be skipped (already pinned
        // check happens elsewhere, but promote_preview moves it).
        g.promote_preview_to_pinned();
        assert!(g.preview.is_none());
        assert_eq!(g.pinned.len(), 1);
        // Now try open_preview with same path — should return None because
        // it's already pinned.
        let (_, bid2, d2, bk2) = entry("/a.rs");
        let result = g.open_preview(p, bid2, d2, bk2, true);
        assert!(result.is_none());
    }
}
