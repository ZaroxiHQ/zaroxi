/*!
Workbench destinations — the single routing concept that drives the rendered
shell when an activity-rail item is selected.

A `WorkbenchDestination` is derived from `GuiApp::rail_selected_index` once per
frame and then drives, coherently and visibly:

- the **sidebar** content ([`sidebar_rows`] + [`WorkbenchDestination::sidebar_title`]),
- the **editor region** (file editor for `Explorer`, otherwise a cockpit page),
- the **tab strip / breadcrumb** label ([`WorkbenchDestination::title`]),
- the **cockpit panels** (Settings / Extensions / placeholder), which read the
  same mock data exposed here so the sidebar and the main pane never disagree.

The Extensions list and Settings categories are backed by a small local mock
model ([`extension_entries`], [`settings_sections`]). A future theme/extension
system can replace these with real providers without changing the render
contract.
*/

use zaroxi_interface_widgets::{ExtensionEntry, SettingsRow, SettingsRowKind, SettingsSection};

/// A first-class destination selected from the activity rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkbenchDestination {
    /// File tree + editor cockpit (the default).
    Explorer,
    /// Full-text search across the workspace.
    Search,
    /// Source-control (git) changes.
    SourceControl,
    /// Run & debug.
    Debug,
    /// Extensions marketplace / manager.
    Extensions,
    /// Application settings.
    Settings,
    /// Account / sign-in.
    Account,
}

impl WorkbenchDestination {
    /// Map a rail index to a destination. Out-of-range falls back to `Explorer`.
    pub fn from_rail_index(index: usize) -> Self {
        match index {
            0 => Self::Explorer,
            1 => Self::Search,
            2 => Self::SourceControl,
            3 => Self::Debug,
            4 => Self::Extensions,
            5 => Self::Settings,
            6 => Self::Account,
            _ => Self::Explorer,
        }
    }

    /// Whether this destination is the file-editing Explorer (the only one that
    /// renders the file editor + explorer tree).
    pub fn is_explorer(&self) -> bool {
        matches!(self, Self::Explorer)
    }

    /// The activity-rail index that selects this destination (inverse of
    /// [`from_rail_index`]). Used to keep the rail highlight in sync with the
    /// active tab.
    pub fn rail_index(&self) -> usize {
        match self {
            Self::Explorer => 0,
            Self::Search => 1,
            Self::SourceControl => 2,
            Self::Debug => 3,
            Self::Extensions => 4,
            Self::Settings => 5,
            Self::Account => 6,
        }
    }

    /// Human title used for the tab strip / breadcrumb / main heading.
    pub fn title(&self) -> &'static str {
        match self {
            Self::Explorer => "Explorer",
            Self::Search => "Search",
            Self::SourceControl => "Source Control",
            Self::Debug => "Run & Debug",
            Self::Extensions => "Extensions",
            Self::Settings => "Settings",
            Self::Account => "Account",
        }
    }

    /// Title shown above the destination's sidebar panel.
    pub fn sidebar_title(&self) -> &'static str {
        match self {
            Self::Explorer => "Explorer",
            Self::Search => "Search",
            Self::SourceControl => "Source Control",
            Self::Debug => "Run & Debug",
            Self::Extensions => "Extensions",
            Self::Settings => "Settings",
            Self::Account => "Account",
        }
    }

    /// `(title, subtitle)` for destinations rendered as a titled placeholder in
    /// the editor content region (no bespoke page yet). `None` for destinations
    /// with their own page (Explorer/Settings/Extensions).
    pub fn placeholder(&self) -> Option<(String, String)> {
        let (title, subtitle) = match self {
            Self::Search => ("Search", "Find across files in the workspace."),
            Self::SourceControl => ("Source Control", "Review and stage changes."),
            Self::Debug => ("Run & Debug", "Launch and inspect debug sessions."),
            Self::Account => ("Account", "Sign in to sync settings and extensions."),
            _ => return None,
        };
        Some((title.to_string(), subtitle.to_string()))
    }
}

/// Identity of a workbench tab. The single source of truth for what the center
/// content shows. `Editor` is the file-editor aggregate (kept for backward
/// compatibility with `active_tab` comparisons). `FileBuffer(String)` is a
/// single opened file identified by its stable buffer id string. The other
/// variants are first-class non-file workbench tabs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorkbenchTabId {
    /// The file editor identity (used for the unified Editor role).
    Editor,
    /// A specific opened file, identified by its stable buffer id.
    FileBuffer(String),
    /// A destination landing/root tab (Search / Source Control / Debug /
    /// Extensions / Settings / Account). Explorer never gets a root tab — it is
    /// the editor.
    DestinationRoot(WorkbenchDestination),
    /// A specific Settings category page (index into the settings sections).
    SettingsSection(usize),
    /// A specific extension detail page (by extension id).
    ExtensionDetail(String),
    /// Welcome screen shown when no file is open and no non-file tab is
    /// active. Not closable and never appears in the tab strip — it is the
    /// default cockpit content.
    Welcome,
}

impl WorkbenchTabId {
    /// The workbench area this tab belongs to — drives the sidebar + rail.
    pub fn destination(&self) -> WorkbenchDestination {
        match self {
            Self::Editor | Self::FileBuffer(_) | Self::Welcome => WorkbenchDestination::Explorer,
            Self::DestinationRoot(d) => *d,
            Self::SettingsSection(_) => WorkbenchDestination::Settings,
            Self::ExtensionDetail(_) => WorkbenchDestination::Extensions,
        }
    }

    /// Whether this tab shows the file editor (vs a cockpit destination page).
    /// Only `Editor` (the sentinel) and the Explorer destination root are
    /// considered editor-mode. `FileBuffer(_)` is a concrete file identity,
    /// not a mode sentinel — use [`is_file_buffer`] for that.
    pub fn is_editor(&self) -> bool {
        matches!(self, Self::Editor | Self::DestinationRoot(WorkbenchDestination::Explorer))
    }

    /// Whether this is a concrete opened-file tab identity (not a sentinel).
    pub fn is_file_buffer(&self) -> bool {
        matches!(self, Self::FileBuffer(_))
    }
}

/// An open non-file workbench tab (file tabs are projected from the buffer
/// summary each frame and never stored here).
#[derive(Debug, Clone)]
pub struct WorkbenchTab {
    /// Stable identity (used for focus + dedup).
    pub id: WorkbenchTabId,
    /// Visible tab label, e.g. "Settings: General", "Zaroxi Formatter".
    pub title: String,
}

/// A rendered tab's hit region, set each frame from the cockpit tab-strip
/// layout so the host can route clicks (focus / close) without the widget tree.
#[derive(Debug, Clone)]
pub struct WorkbenchTabHit {
    /// Whole-tab hit rect `(x, y, w, h)`.
    pub rect: (f32, f32, f32, f32),
    /// Close-button hit rect, when the tab is closable.
    pub close_rect: Option<(f32, f32, f32, f32)>,
    /// Stable tab identity. File tabs use `FileBuffer(buffer_id_string)`,
    /// non-file tabs use `DestinationRoot` / `SettingsSection` / `ExtensionDetail`.
    pub id: WorkbenchTabId,
}

/// Title for a non-file tab id (used when opening a tab).
pub fn tab_title(id: &WorkbenchTabId) -> String {
    match id {
        WorkbenchTabId::Editor => "Editor".to_string(),
        WorkbenchTabId::Welcome => "Welcome".to_string(),
        WorkbenchTabId::FileBuffer(bid) => format!("File {bid}"),
        WorkbenchTabId::DestinationRoot(d) => d.title().to_string(),
        WorkbenchTabId::SettingsSection(i) => {
            let sections = settings_sections();
            let name = sections.get(*i).map(|s| s.label.as_str()).unwrap_or("Settings");
            format!("Settings: {name}")
        }
        WorkbenchTabId::ExtensionDetail(ext_id) => extension_entries()
            .into_iter()
            .find(|e| &e.id == ext_id)
            .map(|e| e.name)
            .unwrap_or_else(|| "Extension".to_string()),
    }
}

/// A canonical entry in the workbench tab strip — either a file buffer tab or
/// a non-file workbench tab. This is the single authoritative tab model; every
/// visual and interactive element in the tab strip must derive from this.
#[derive(Debug, Clone)]
pub enum WorkbenchTabEntry {
    File { buffer_id: String, title: String, is_active_buffer: bool },
    NonFile { id: WorkbenchTabId, title: String },
}

impl WorkbenchTabEntry {
    pub fn stable_id(&self) -> WorkbenchTabId {
        match self {
            Self::File { buffer_id, .. } => WorkbenchTabId::FileBuffer(buffer_id.clone()),
            Self::NonFile { id, .. } => id.clone(),
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::File { title, .. } => title,
            Self::NonFile { title, .. } => title,
        }
    }

    pub fn closable(&self) -> bool {
        true
    }

    pub fn is_file(&self) -> bool {
        matches!(self, Self::File { .. })
    }
}

/// The single canonical tab state for the workbench. Owns all tab identities,
/// the active tab, and scroll position. Every open, close, focus, and scroll
/// action must go through one of its methods — no alternative authority exists.
///
/// File tabs are synced from composition metadata each frame via
/// [`sync_file_tabs`]; non-file tabs are directly owned here.
#[derive(Debug, Clone)]
pub struct WorkbenchTabState {
    entries: Vec<WorkbenchTabEntry>,
    active: WorkbenchTabId,
    pub scroll_offset: f32,
}

impl Default for WorkbenchTabState {
    fn default() -> Self {
        Self {
            entries: vec![WorkbenchTabEntry::NonFile {
                id: WorkbenchTabId::Welcome,
                title: "Welcome".into(),
            }],
            active: WorkbenchTabId::Welcome,
            scroll_offset: 0.0,
        }
    }
}

impl WorkbenchTabState {
    pub fn new() -> Self {
        Self::default()
    }

    /// The ordered set of canonical tab entries.
    pub fn entries(&self) -> &[WorkbenchTabEntry] {
        &self.entries
    }

    /// The active tab identity.
    pub fn active(&self) -> &WorkbenchTabId {
        &self.active
    }

    /// Sync file tabs from the composition's opened-buffers summary.
    /// Rebuilds the file-tab portion of the entry list while preserving
    /// all non-file tabs in their current order. Duplicates (same buffer_id
    /// or same file path) are removed — only the first occurrence is kept.
    pub fn sync_file_tabs(&mut self, titles: &[(String, String, bool)]) {
        self.entries.retain(|e| !e.is_file());
        let all_paths: Vec<&str> = titles.iter().map(|(t, _, _)| t.as_str()).collect();
        let mut seen_bid = std::collections::HashSet::new();
        let mut seen_display = std::collections::HashSet::new();
        let mut new_files: Vec<WorkbenchTabEntry> = Vec::with_capacity(titles.len());
        for (path, bid, is_active) in titles.iter() {
            if !seen_bid.insert(bid.as_str()) {
                continue;
            }
            if !seen_display.insert(path.as_str()) {
                continue;
            }
            new_files.push(WorkbenchTabEntry::File {
                buffer_id: bid.clone(),
                title: format_file_tab_label(path, &all_paths),
                is_active_buffer: *is_active,
            });
        }
        let prev_entries = self.entries.len();
        self.entries = new_files.drain(..).chain(self.entries.drain(..)).collect();
        if std::env::var("ZAROXI_DOC_LIFECYCLE").as_deref() == Ok("1") {
            eprintln!(
                "ZAROXI_DOC_LIFECYCLE: tab_sync file_tabs={} non_file_tabs={} total={} prev_non_file={} active={:?}",
                titles.len(),
                self.entries.len().saturating_sub(titles.len()),
                self.entries.len(),
                prev_entries,
                self.active,
            );
        }
    }

    /// Open or focus a non-file tab. Deduplicates by id; if the tab already
    /// exists it is focused, otherwise it is created and made active.
    /// File-buffer identities are not handled here — they are owned by the
    /// composition metadata and synced via [`sync_file_tabs`].
    pub fn open_or_focus_non_file(&mut self, id: WorkbenchTabId) {
        if id.is_editor() || id.is_file_buffer() {
            self.active = WorkbenchTabId::Editor;
            return;
        }
        // Create or focus the non-file tab. Welcome is treated as a normal
        // non-file entry — it appears in the strip and can be closed.
        if let Some(existing) = self.entries.iter().position(|e| e.stable_id() == id) {
            self.active = self.entries[existing].stable_id();
        } else {
            let title = tab_title(&id);
            self.entries.push(WorkbenchTabEntry::NonFile { id: id.clone(), title });
            self.active = id;
        }
    }

    /// Close a tab by its stable identity. Returns `true` if the active tab
    /// changed and the caller needs to update content.
    pub fn close_tab(&mut self, id: &WorkbenchTabId) -> bool {
        let Some(pos) = self.entries.iter().position(|e| &e.stable_id() == id) else {
            return false;
        };
        let was_file = self.entries[pos].is_file();
        let was_active = &self.active == id;
        if std::env::var("ZAROXI_DOC_LIFECYCLE").as_deref() == Ok("1") {
            eprintln!(
                "ZAROXI_DOC_LIFECYCLE: tab_close id={:?} was_file={} was_active={} pos={} entries_before={}",
                id,
                was_file,
                was_active,
                pos,
                self.entries.len(),
            );
        }
        self.entries.remove(pos);
        if was_active {
            // Prefer successor at same slot, then predecessor, then Editor.
            let fallback = self
                .entries
                .get(pos)
                .or_else(|| pos.checked_sub(1).and_then(|p| self.entries.get(p)))
                .map(|e| e.stable_id())
                .unwrap_or(WorkbenchTabId::Editor);
            if std::env::var("ZAROXI_DOC_LIFECYCLE").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_DOC_LIFECYCLE: tab_close_fallback active_was={:?} active_now={:?} entries_after={}",
                    id,
                    fallback,
                    self.entries.len(),
                );
            }
            self.active = fallback;
        }
        was_file || was_active
    }

    /// Focus an existing tab by id. Returns `true` if the active tab changed.
    /// File-buffer identities are translated to the `Editor` sentinel so the
    /// active mode is correctly set to file-editor.
    pub fn focus_tab(&mut self, id: &WorkbenchTabId) -> bool {
        if self.active == *id {
            return false;
        }
        if id.is_editor() || id.is_file_buffer() {
            self.active = WorkbenchTabId::Editor;
            return true;
        }
        if self.entries.iter().any(|e| e.stable_id() == *id) {
            self.active = id.clone();
            return true;
        }
        false
    }

    /// Ensure the active tab is visible within `strip_w` logical pixels.
    /// Nudges `scroll_offset` so the active tab is fully inside the viewport.
    pub fn ensure_active_visible(&mut self, strip_w: f32, tab_w: f32) {
        if let Some(act_idx) = self.entries.iter().position(|e| e.stable_id() == self.active) {
            let tab_left = act_idx as f32 * tab_w;
            let tab_right = tab_left + tab_w;
            if tab_right - self.scroll_offset > strip_w {
                self.scroll_offset = (tab_right - strip_w + 4.0).max(0.0);
            }
            if tab_left < self.scroll_offset {
                self.scroll_offset = (tab_left - 4.0).max(0.0);
            }
        }
        let total_w = self.entries.len() as f32 * tab_w;
        let max_scroll = (total_w - strip_w).max(0.0);
        if max_scroll <= 0.0 {
            self.scroll_offset = 0.0;
        } else {
            self.scroll_offset = self.scroll_offset.min(max_scroll);
        }
    }

    /// Whether the active tab is in file-editor mode.
    pub fn is_editor_active(&self) -> bool {
        self.active.is_editor()
    }

    /// Project the canonical tab list into a `Vec<UnifiedTab>` for cockpit
    /// consumption and hit-test layout.
    ///
    /// File tabs are derived EXCLUSIVELY from `editor_group.visible_tabs()`.
    /// No file tab may be rendered from `opened_buffers`, `sync_file_tabs`,
    /// or any other legacy source.  Non-file tabs (Settings, Extensions,
    /// Welcome) remain owned by [`WorkbenchTabState::entries`].
    pub fn projected_tabs(&self, editor_group: &super::EditorGroup) -> Vec<UnifiedTab> {
        let editor_active = self.active.is_editor();

        // File tabs — single source of truth: EditorGroup.
        let tabs: Vec<UnifiedTab> = editor_group
            .visible_tabs()
            .into_iter()
            .map(|vt| UnifiedTab {
                title: vt.display,
                active: vt.is_active && editor_active,
                closable: true,
                id: WorkbenchTabId::FileBuffer(vt.buffer_id),
                kind: zaroxi_interface_widgets::TabKind::File,
                is_preview: vt.is_preview,
            })
            .collect();

        if std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1") {
            let rendered = tabs
                .iter()
                .map(|t| {
                    let path = match &t.id {
                        WorkbenchTabId::FileBuffer(b) => b.strip_prefix("buf:").unwrap_or(b),
                        _ => "?",
                    };
                    format!(
                        "{{path={} title={} is_preview={} is_active={}}}",
                        path, t.title, t.is_preview, t.active,
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            // Change-detection gate: this projection is rebuilt every frame,
            // so emit only when the rendered set actually changes.  Prevents
            // per-frame `file_tab_projection` log spam for an unchanged strip.
            thread_local! {
                static LAST_PROJECTION: std::cell::RefCell<Option<String>> =
                    const { std::cell::RefCell::new(None) };
            }
            let changed = LAST_PROJECTION.with(|last| {
                let mut last = last.borrow_mut();
                if last.as_deref() == Some(rendered.as_str()) {
                    false
                } else {
                    *last = Some(rendered.clone());
                    true
                }
            });
            if changed {
                eprintln!(
                    "ZAROXI_VISIBLE_TAB_MODEL: projection_emitted changed=true file_tab_projection source=editor_group tabs=[{}]",
                    rendered,
                );
            }
        }

        tabs
    }
}

/// One row in a destination's sidebar list.
#[derive(Debug, Clone)]
pub struct DestSidebarRow {
    /// Primary label (extension name, settings category, etc.).
    pub label: String,
    /// Trailing badge / hint (e.g. "Installed", "Available", or empty).
    pub secondary: String,
    /// Whether this row is the active selection (drives the highlight + detail).
    pub selected: bool,
    /// Whether clicking this row changes the active selection (Extensions /
    /// Settings). Decorative rows for other destinations are not selectable.
    pub selectable: bool,
}

impl DestSidebarRow {
    fn new(label: &str, secondary: &str, selected: bool, selectable: bool) -> Self {
        Self { label: label.to_string(), secondary: secondary.to_string(), selected, selectable }
    }
}

/// Mock extension catalogue. A future extension system replaces this provider.
pub fn extension_entries() -> Vec<ExtensionEntry> {
    vec![
        ExtensionEntry {
            id: "zaroxi.formatter".into(),
            name: "Zaroxi Formatter".into(),
            publisher: "Zaroxi Team".into(),
            description: "Code formatting for Rust, TOML, JSON, and more.".into(),
            installed: true,
        },
        ExtensionEntry {
            id: "zaroxi.lsp-client".into(),
            name: "LSP Client".into(),
            publisher: "Zaroxi Team".into(),
            description: "Language Server Protocol client for Rust, TypeScript, Python.".into(),
            installed: true,
        },
        ExtensionEntry {
            id: "community.themes".into(),
            name: "Community Themes".into(),
            publisher: "Community".into(),
            description: "A collection of popular color themes.".into(),
            installed: false,
        },
        ExtensionEntry {
            id: "zaroxi.git".into(),
            name: "Git Integration".into(),
            publisher: "Zaroxi Team".into(),
            description: "Inline blame, diff gutters, and commit tooling.".into(),
            installed: false,
        },
    ]
}

/// Mock settings sections. The structure (sections, row labels, descriptions)
/// is defined here; live values come from the `Settings` state in `CockpitInputs`.
/// Theme and font rows use dedicated `SettingsRowKind::Theme` / `::Font` variants
/// so the panel renders interactive pickers and the host can dispatch
/// `SettingsAction` on click.
pub fn settings_sections() -> Vec<SettingsSection> {
    vec![
        SettingsSection {
            label: "General".to_string(),
            items: vec![
                SettingsRow {
                    label: "Color Theme".to_string(),
                    description: "Choose the editor color theme".to_string(),
                    kind: SettingsRowKind::Theme,
                },
                SettingsRow {
                    label: "Editor Font".to_string(),
                    description: "Monospace font for editor and UI text".to_string(),
                    kind: SettingsRowKind::Font,
                },
            ],
        },
        SettingsSection {
            label: "Privacy".to_string(),
            items: vec![SettingsRow {
                label: "Telemetry".to_string(),
                description: "Send anonymous usage data to improve Zaroxi".to_string(),
                kind: SettingsRowKind::Toggle { on: false },
            }],
        },
        SettingsSection {
            label: "Editor".to_string(),
            items: vec![
                SettingsRow {
                    label: "Tab Size".to_string(),
                    description: "Spaces per tab".to_string(),
                    kind: SettingsRowKind::Label { value: "4".to_string() },
                },
                SettingsRow {
                    label: "Word Wrap".to_string(),
                    description: "Wrap long lines to the viewport".to_string(),
                    kind: SettingsRowKind::Toggle { on: false },
                },
            ],
        },
        SettingsSection {
            label: "Appearance".to_string(),
            items: vec![
                SettingsRow {
                    label: "Font Size".to_string(),
                    description: "Default editor font size in pixels".to_string(),
                    kind: SettingsRowKind::Label { value: "13 px".to_string() },
                },
                SettingsRow {
                    label: "Minimap".to_string(),
                    description: "Show the semantic minimap".to_string(),
                    kind: SettingsRowKind::Toggle { on: true },
                },
            ],
        },
        SettingsSection {
            label: "Keybindings".to_string(),
            items: vec![SettingsRow {
                label: "Scheme".to_string(),
                description: "Keyboard shortcut scheme".to_string(),
                kind: SettingsRowKind::Select {
                    value: "Default".to_string(),
                    options: vec!["Default".to_string(), "Vim".to_string()],
                },
            }],
        },
    ]
}

/// Build the destination's sidebar rows. For Extensions/Settings these are the
/// selectable list driving the detail pane; for the placeholder destinations
/// they are decorative facets so the sidebar visibly changes per destination.
pub fn sidebar_rows(
    dest: WorkbenchDestination,
    extensions_selected: Option<usize>,
    settings_selected: Option<usize>,
) -> Vec<DestSidebarRow> {
    match dest {
        WorkbenchDestination::Explorer => Vec::new(),
        WorkbenchDestination::Extensions => extension_entries()
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let badge = if e.installed { "Installed" } else { "Available" };
                DestSidebarRow::new(&e.name, badge, Some(i) == extensions_selected, true)
            })
            .collect(),
        WorkbenchDestination::Settings => settings_sections()
            .iter()
            .enumerate()
            .map(|(i, s)| DestSidebarRow::new(&s.label, "", Some(i) == settings_selected, true))
            .collect(),
        WorkbenchDestination::Search => vec![
            DestSidebarRow::new("Match Case", "Aa", false, false),
            DestSidebarRow::new("Whole Word", "ab", false, false),
            DestSidebarRow::new("Use Regex", ".*", false, false),
        ],
        WorkbenchDestination::SourceControl => vec![
            DestSidebarRow::new("Changes", "0", false, false),
            DestSidebarRow::new("Staged Changes", "0", false, false),
            DestSidebarRow::new("Commits", "", false, false),
        ],
        WorkbenchDestination::Debug => vec![
            DestSidebarRow::new("Run and Debug", "", false, false),
            DestSidebarRow::new("Breakpoints", "0", false, false),
            DestSidebarRow::new("Watch", "", false, false),
        ],
        WorkbenchDestination::Account => vec![
            DestSidebarRow::new("Sign In", "", false, false),
            DestSidebarRow::new("Profile", "", false, false),
            DestSidebarRow::new("Sync Settings", "Off", false, false),
        ],
    }
}

/// A tab in the unified tab strip (file tabs + non-file workbench tabs).
#[derive(Debug, Clone)]
pub struct UnifiedTab {
    /// Visible label.
    pub title: String,
    /// Whether this is the active tab.
    pub active: bool,
    /// Whether a close button is shown (non-file tabs are closable).
    pub closable: bool,
    /// Stable tab identity. File tabs use `FileBuffer(id_string)`, non-file
    /// tabs use `DestinationRoot` / `SettingsSection` / etc.
    pub id: WorkbenchTabId,
    /// Which zone this tab belongs to.
    pub kind: zaroxi_interface_widgets::TabKind,
    /// True when this tab is the shared preview (italic label).
    pub is_preview: bool,
}

/// Format a file path into a compact, filename-first tab label.
///
/// Rules (matches common IDE tab-label conventions):
/// - Single file: basename only (e.g. `AGENTS.md`)
/// - Unique basenames among all open files: basename only
/// - Duplicate basenames: add one parent-dir segment (e.g. `src/main.rs`,
///   `client/main.rs`)
/// - Paths that look like non-filesystem display names are returned as-is.
pub fn format_file_tab_label(path: &str, all_paths: &[&str]) -> String {
    // Guard: paths without any separator are likely display names, not
    // filesystem paths. Return them unchanged.
    if !path.contains('/') && !path.contains('\\') {
        return path.to_string();
    }

    let basename = std::path::Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or(path);

    // Collect all basenames from sibling paths.
    let same_basename_count = all_paths
        .iter()
        .filter(|p| {
            std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|b| b == basename)
                .unwrap_or(false)
        })
        .count();

    // Unique — just the basename.
    if same_basename_count <= 1 {
        return basename.to_string();
    }

    // Duplicate: add one parent segment for disambiguation.
    let parent = std::path::Path::new(path)
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if parent.is_empty() { format!("../{basename}") } else { format!("{parent}/{basename}") }
}

/// Project the primary editor tab strip — file tabs only.
/// Non-file destinations (Settings, Extensions, AI Assistant) are accessed
/// exclusively through the activity rail and do NOT appear in the tab strip.
/// Welcome is rendered as an editor-surface empty state, not as a tab.
///
/// If `preview_path` is set and matches a file in `file_tabs`, that entry
/// is marked `is_preview: true`.  If `preview_path` is set but the file is
/// NOT already a pinned tab, a synthetic preview entry is injected at the
/// front of the list.
pub fn build_unified_tabs(
    _file_tabs: &[(String, String, bool)],
    active_tab: &WorkbenchTabId,
    _non_file_tabs: &[WorkbenchTab],
    editor_group: &super::EditorGroup,
) -> Vec<UnifiedTab> {
    let editor_active = active_tab.is_editor();
    editor_group
        .visible_tabs()
        .into_iter()
        .map(|vt| UnifiedTab {
            title: vt.display,
            active: vt.is_active && editor_active,
            closable: true,
            id: WorkbenchTabId::FileBuffer(vt.buffer_id),
            kind: zaroxi_interface_widgets::TabKind::File,
            is_preview: vt.is_preview,
        })
        .collect()
}

/// Resolve the cockpit destination pages for the active tab: returns
/// `(settings_panel, extensions_panel, placeholder_panel)`. Exactly one (or
/// none, for the editor) is `Some`.
#[allow(clippy::type_complexity)]
pub fn cockpit_panels_for(
    active: &WorkbenchTabId,
) -> (
    Option<(Vec<SettingsSection>, usize)>,
    Option<(Vec<ExtensionEntry>, usize)>,
    Option<(String, String)>,
) {
    match active {
        WorkbenchTabId::Editor | WorkbenchTabId::FileBuffer(_) => (None, None, None),
        // Welcome renders via WelcomePanel widget, not via placeholder.
        WorkbenchTabId::Welcome => (None, None, None),
        WorkbenchTabId::SettingsSection(i) => {
            let sections = settings_sections();
            let sel = (*i).min(sections.len().saturating_sub(1));
            (Some((sections, sel)), None, None)
        }
        WorkbenchTabId::DestinationRoot(WorkbenchDestination::Settings) => {
            (Some((settings_sections(), 0)), None, None)
        }
        WorkbenchTabId::ExtensionDetail(id) => {
            let entries = extension_entries();
            let sel = entries.iter().position(|e| &e.id == id).unwrap_or(0);
            (None, Some((entries, sel)), None)
        }
        WorkbenchTabId::DestinationRoot(WorkbenchDestination::Extensions) => {
            (None, Some((extension_entries(), 0)), None)
        }
        WorkbenchTabId::DestinationRoot(d) => (None, None, d.placeholder()),
    }
}

/// Sidebar selection highlight `(extensions_selected, settings_selected)`
/// derived from the active tab — no separate selection state is stored.
pub fn sidebar_selection_for(active: &WorkbenchTabId) -> (Option<usize>, Option<usize>) {
    match active {
        WorkbenchTabId::ExtensionDetail(id) => {
            (extension_entries().iter().position(|e| &e.id == id), None)
        }
        WorkbenchTabId::SettingsSection(i) => (None, Some(*i)),
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rail_index_maps_to_destination() {
        assert_eq!(WorkbenchDestination::from_rail_index(0), WorkbenchDestination::Explorer);
        assert_eq!(WorkbenchDestination::from_rail_index(4), WorkbenchDestination::Extensions);
        assert_eq!(WorkbenchDestination::from_rail_index(5), WorkbenchDestination::Settings);
        // Out of range falls back to Explorer.
        assert_eq!(WorkbenchDestination::from_rail_index(99), WorkbenchDestination::Explorer);
    }

    #[test]
    fn only_explorer_is_explorer() {
        assert!(WorkbenchDestination::Explorer.is_explorer());
        assert!(!WorkbenchDestination::Settings.is_explorer());
        assert!(!WorkbenchDestination::Extensions.is_explorer());
    }

    #[test]
    fn placeholder_only_for_facet_destinations() {
        assert!(WorkbenchDestination::Search.placeholder().is_some());
        assert!(WorkbenchDestination::Account.placeholder().is_some());
        // Pages with bespoke views have no placeholder.
        assert!(WorkbenchDestination::Explorer.placeholder().is_none());
        assert!(WorkbenchDestination::Settings.placeholder().is_none());
        assert!(WorkbenchDestination::Extensions.placeholder().is_none());
    }

    #[test]
    fn extensions_sidebar_marks_selection() {
        let rows = sidebar_rows(WorkbenchDestination::Extensions, Some(1), None);
        assert!(rows[1].selected);
        assert!(rows[0].selectable);
        assert!(!rows[0].selected);
    }

    #[test]
    fn explorer_has_no_destination_rows() {
        assert!(sidebar_rows(WorkbenchDestination::Explorer, None, None).is_empty());
    }

    #[test]
    fn unified_tabs_only_file_tabs_and_flags_active() {
        // Build an EditorGroup with two pinned files.
        let mut eg = super::super::EditorGroup::default();
        eg.open_or_activate_pinned(
            "src/main.rs".into(),
            "buf_1".into(),
            "main.rs".into(),
            super::super::BackendKind::Rope,
            true,
        );
        eg.open_or_activate_pinned(
            "src/lib.rs".into(),
            "buf_2".into(),
            "lib.rs".into(),
            super::super::BackendKind::Rope,
            true,
        );
        // Activate the first.
        eg.activate_by_path("src/main.rs");

        let non_file = vec![
            WorkbenchTab {
                id: WorkbenchTabId::DestinationRoot(WorkbenchDestination::Settings),
                title: "Settings".to_string(),
            },
            WorkbenchTab {
                id: WorkbenchTabId::ExtensionDetail("zaroxi.git".to_string()),
                title: "Git Integration".to_string(),
            },
        ];

        let tabs = build_unified_tabs(&[], &WorkbenchTabId::Editor, &non_file, &eg);
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].title, "main.rs");
        assert!(tabs[0].active && tabs[0].closable);
        assert!(!tabs[0].is_preview);
        assert!(matches!(tabs[0].id, WorkbenchTabId::FileBuffer(_)));
        assert!(!tabs[1].active);

        let ext = WorkbenchTabId::ExtensionDetail("zaroxi.git".to_string());
        let tabs = build_unified_tabs(&[], &ext, &non_file, &eg);
        assert_eq!(tabs.len(), 2);
        assert!(!tabs[0].active && !tabs[1].active);
    }

    #[test]
    fn format_file_tab_disambiguates_duplicates() {
        let paths: Vec<&str> = vec!["src/main.rs", "client/main.rs", "src/lib.rs"];
        assert_eq!(format_file_tab_label("src/main.rs", &paths), "src/main.rs");
        assert_eq!(format_file_tab_label("client/main.rs", &paths), "client/main.rs");
        assert_eq!(format_file_tab_label("src/lib.rs", &paths), "lib.rs");
    }

    #[test]
    fn format_file_tab_uses_basename_for_unique_names() {
        let paths: Vec<&str> = vec!["src/main.rs", "src/lib.rs"];
        assert_eq!(format_file_tab_label("src/main.rs", &paths), "main.rs");
        assert_eq!(format_file_tab_label("AGENTS.md", &paths), "AGENTS.md");
    }

    #[test]
    fn panels_follow_active_tab() {
        // Settings section -> settings panel at that index.
        let (s, e, p) = cockpit_panels_for(&WorkbenchTabId::SettingsSection(2));
        assert!(s.is_some() && e.is_none() && p.is_none());
        assert_eq!(s.unwrap().1, 2);

        // Extension detail -> extensions panel selecting that id.
        let id = extension_entries()[2].id.clone();
        let (s, e, _p) = cockpit_panels_for(&WorkbenchTabId::ExtensionDetail(id));
        assert!(s.is_none());
        assert_eq!(e.unwrap().1, 2);

        // A facet destination root -> placeholder; the editor -> nothing.
        let (_s, _e, p) =
            cockpit_panels_for(&WorkbenchTabId::DestinationRoot(WorkbenchDestination::Search));
        assert!(p.is_some());
        let (s, e, p) = cockpit_panels_for(&WorkbenchTabId::Editor);
        assert!(s.is_none() && e.is_none() && p.is_none());
    }

    #[test]
    fn sidebar_selection_derives_from_active_tab() {
        let id = extension_entries()[1].id.clone();
        assert_eq!(sidebar_selection_for(&WorkbenchTabId::ExtensionDetail(id)), (Some(1), None));
        assert_eq!(sidebar_selection_for(&WorkbenchTabId::SettingsSection(3)), (None, Some(3)));
        assert_eq!(sidebar_selection_for(&WorkbenchTabId::Editor), (None, None));
    }

    #[test]
    fn tab_titles_are_human_readable() {
        assert_eq!(
            tab_title(&WorkbenchTabId::DestinationRoot(WorkbenchDestination::Extensions)),
            "Extensions"
        );
        assert!(tab_title(&WorkbenchTabId::SettingsSection(0)).starts_with("Settings: "));
        let id = extension_entries()[0].id.clone();
        assert_eq!(tab_title(&WorkbenchTabId::ExtensionDetail(id)), extension_entries()[0].name);
    }
}
