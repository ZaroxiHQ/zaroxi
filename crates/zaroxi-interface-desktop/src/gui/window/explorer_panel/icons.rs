/*!
Explorer row iconography.

A tiny, centralized mapping from (row kind, expand state) to the glyphs used in
the file tree: a disclosure chevron for directories and a type icon for every
row. Glyphs are JetBrains Mono Nerd Font (Font Awesome) code points for now;
keeping them in this one place means a future custom icon set can be swapped in
by editing only `glyph` and `row_label` — explorer layout/logic stays untouched.
*/

/// Raw glyph code points. Replace these with a custom icon set later.
pub mod glyph {
    /// Disclosure marker for an expanded directory (▼).
    pub const CHEVRON_EXPANDED: char = '\u{25BC}';
    /// Disclosure marker for a collapsed directory (▶).
    pub const CHEVRON_COLLAPSED: char = '\u{25B6}';
    /// Closed/collapsed directory icon — nf-fa-folder.
    pub const FOLDER: char = '\u{f07b}';
    /// Open/expanded directory icon — nf-fa-folder_open.
    pub const FOLDER_OPEN: char = '\u{f07c}';
    /// Generic file icon — nf-fa-file.
    pub const FILE: char = '\u{f15b}';
}

/// Build the display label for one explorer row.
///
/// Directory rows are `"<chevron> <folder> <name>"`; file rows are
/// `"  <file> <name>"` — the two leading spaces stand in for the (absent)
/// chevron so the icon column lines up with directory rows at the same depth.
pub fn row_label(is_dir: bool, expanded: bool, name: &str) -> String {
    if is_dir {
        let chevron = if expanded { glyph::CHEVRON_EXPANDED } else { glyph::CHEVRON_COLLAPSED };
        let folder = if expanded { glyph::FOLDER_OPEN } else { glyph::FOLDER };
        format!("{chevron} {folder} {name}")
    } else {
        format!("  {} {name}", glyph::FILE)
    }
}
