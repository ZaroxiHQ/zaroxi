/*!
Explorer row iconography.

Centralized mapping from a tree row to the glyphs used in the file tree:

- a disclosure chevron for directories,
- a type icon chosen by special filename → extension → category fallback.

Glyphs are JetBrains Mono Nerd Font code points for now. Keeping every glyph
literal and the whole mapping in this one module means a future custom icon set
can be swapped in by editing only `glyph` + the mapping functions here —
explorer layout, view-model, and render code stay untouched.

The renderer draws the disclosure+icon column and the filename in *separate*
fixed-x columns (see `rail.rs`), so a double-width Nerd Font icon can never push
the filename column out of alignment. This module only decides *which* glyphs to
show; column geometry lives in `editor_shell::constants`.
*/

/// Raw glyph code points. Replace these with a custom icon set later.
pub mod glyph {
    // ── Disclosure ──
    /// Expanded directory disclosure (▼) — a 1-cell geometric glyph.
    pub const CHEVRON_EXPANDED: char = '\u{25BC}';
    /// Collapsed directory disclosure (▶) — a 1-cell geometric glyph.
    pub const CHEVRON_COLLAPSED: char = '\u{25B6}';

    // ── Directories ──
    /// Closed directory — nf-fa-folder.
    pub const FOLDER: char = '\u{f07b}';
    /// Open directory — nf-fa-folder_open.
    pub const FOLDER_OPEN: char = '\u{f07c}';
    /// Magnifying glass for the search box — nf-fa-search.
    pub const SEARCH: char = '\u{f002}';

    // ── Category fallbacks ──
    /// Generic file — nf-fa-file.
    pub const FILE: char = '\u{f15b}';
    /// Source/code fallback — nf-fa-code.
    pub const CODE: char = '\u{f121}';
    /// Config fallback — nf-fa-cog.
    pub const CONFIG: char = '\u{f013}';
    /// Docs/plain-text fallback — nf-fa-file_text.
    pub const TEXT: char = '\u{f15c}';
    /// Image/media fallback — nf-fa-file_image.
    pub const IMAGE: char = '\u{f1c5}';
    /// Archive/binary fallback — nf-fa-file_archive.
    pub const ARCHIVE: char = '\u{f1c6}';

    // ── Language / type specific ──
    pub const RUST: char = '\u{e7a8}';
    pub const GIT: char = '\u{e702}';
    pub const NIX: char = '\u{f313}';
    pub const MARKDOWN: char = '\u{f48a}';
    pub const JSON: char = '\u{e60b}';
    pub const YAML: char = '\u{e615}';
    pub const LOCK: char = '\u{f023}';
    pub const SHELL: char = '\u{f489}';
    pub const JAVASCRIPT: char = '\u{e74e}';
    pub const TYPESCRIPT: char = '\u{e628}';
    pub const REACT: char = '\u{e7ba}';
    pub const HTML: char = '\u{e736}';
    pub const CSS: char = '\u{e749}';
    pub const SASS: char = '\u{e74b}';
    pub const PYTHON: char = '\u{e73c}';
    pub const GO: char = '\u{e724}';
    pub const JAVA: char = '\u{e738}';
    pub const C: char = '\u{e61e}';
    pub const CPP: char = '\u{e61d}';
    pub const PDF: char = '\u{f1c1}';
    pub const NPM: char = '\u{e71e}';
    pub const DOCKER: char = '\u{f308}';
}

/// Directory icon (open when expanded).
pub fn directory_icon(expanded: bool) -> char {
    if expanded { glyph::FOLDER_OPEN } else { glyph::FOLDER }
}

/// Disclosure chevron for a directory.
pub fn chevron(expanded: bool) -> char {
    if expanded { glyph::CHEVRON_EXPANDED } else { glyph::CHEVRON_COLLAPSED }
}

/// Pick a type icon for a file by name.
///
/// Resolution order: exact special filename → extension → category fallback →
/// generic file. Case-insensitive throughout, so `Cargo.toml`, `cargo.toml`,
/// and dotfiles like `.gitignore` all resolve.
pub fn file_icon(name: &str) -> char {
    let lower = name.to_ascii_lowercase();

    // 1. Known special files (exact name).
    match lower.as_str() {
        "cargo.toml" | "cargo.lock" => return glyph::RUST,
        ".gitignore" | ".gitattributes" | ".gitmodules" | ".gitconfig" => return glyph::GIT,
        "flake.nix" | "flake.lock" | "shell.nix" | "default.nix" => return glyph::NIX,
        "package.json" | "package-lock.json" | ".npmrc" => return glyph::NPM,
        "dockerfile" | ".dockerignore" | "docker-compose.yml" | "docker-compose.yaml" => {
            return glyph::DOCKER;
        }
        "readme" | "readme.md" | "readme.txt" => return glyph::MARKDOWN,
        ".envrc" | ".env" | "makefile" | "justfile" | ".editorconfig" => return glyph::CONFIG,
        _ => {}
    }

    // 2. Extension.
    if let Some(ext) = lower.rsplit_once('.').map(|(_, e)| e) {
        match ext {
            "rs" => return glyph::RUST,
            "toml" => return glyph::YAML,
            "md" | "markdown" => return glyph::MARKDOWN,
            "txt" | "text" => return glyph::TEXT,
            "json" | "jsonc" => return glyph::JSON,
            "yaml" | "yml" => return glyph::YAML,
            "nix" => return glyph::NIX,
            "lock" => return glyph::LOCK,
            "sh" | "bash" | "zsh" | "fish" => return glyph::SHELL,
            "js" | "mjs" | "cjs" => return glyph::JAVASCRIPT,
            "ts" => return glyph::TYPESCRIPT,
            "jsx" | "tsx" => return glyph::REACT,
            "html" | "htm" => return glyph::HTML,
            "css" => return glyph::CSS,
            "scss" | "sass" => return glyph::SASS,
            "py" | "pyi" | "pyw" => return glyph::PYTHON,
            "go" => return glyph::GO,
            "java" | "kt" | "kts" => return glyph::JAVA,
            "c" | "h" => return glyph::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => return glyph::CPP,
            "svg" | "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "ico" => {
                return glyph::IMAGE;
            }
            "pdf" => return glyph::PDF,
            "zip" | "tar" | "gz" | "tgz" | "xz" | "bz2" | "7z" | "rar" | "zst" => {
                return glyph::ARCHIVE;
            }
            _ => {}
        }
    }

    glyph::FILE
}

/// The disclosure + type-icon string for a row's glyph column (everything left
/// of the filename). Files reserve a 2-cell prefix so their type icon lines up
/// with directory icons (which sit after a 1-cell chevron + space).
pub fn glyph_prefix(is_dir: bool, expanded: bool, name: &str) -> String {
    if is_dir {
        format!("{} {}", chevron(expanded), directory_icon(expanded))
    } else {
        format!("  {}", file_icon(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_icons_resolve() {
        assert_eq!(file_icon("main.rs"), glyph::RUST);
        assert_eq!(file_icon("README.md"), glyph::MARKDOWN);
        assert_eq!(file_icon("config.yaml"), glyph::YAML);
        assert_eq!(file_icon("data.json"), glyph::JSON);
        assert_eq!(file_icon("script.sh"), glyph::SHELL);
        assert_eq!(file_icon("photo.PNG"), glyph::IMAGE);
        assert_eq!(file_icon("bundle.tar.gz"), glyph::ARCHIVE);
        assert_eq!(file_icon("app.tsx"), glyph::REACT);
        assert_eq!(file_icon("mod.cpp"), glyph::CPP);
    }

    #[test]
    fn special_files_resolve_case_insensitively() {
        assert_eq!(file_icon("Cargo.toml"), glyph::RUST);
        assert_eq!(file_icon("cargo.lock"), glyph::RUST);
        assert_eq!(file_icon(".gitignore"), glyph::GIT);
        assert_eq!(file_icon("flake.nix"), glyph::NIX);
        assert_eq!(file_icon(".envrc"), glyph::CONFIG);
        // README without extension still gets the docs icon.
        assert_eq!(file_icon("README"), glyph::MARKDOWN);
    }

    #[test]
    fn unknown_and_extensionless_fall_back_to_generic_file() {
        assert_eq!(file_icon("mystery.qwerty"), glyph::FILE);
        assert_eq!(file_icon("LICENSE"), glyph::FILE);
        assert_eq!(file_icon("noext"), glyph::FILE);
    }

    #[test]
    fn glyph_prefix_aligns_file_icon_column_with_directories() {
        // Directory prefix: chevron + space + folder icon (icon at char index 2).
        let dir = glyph_prefix(true, false, "src");
        let mut dir_chars = dir.chars();
        assert_eq!(dir_chars.next(), Some(glyph::CHEVRON_COLLAPSED));
        assert_eq!(dir_chars.next(), Some(' '));
        assert_eq!(dir_chars.next(), Some(glyph::FOLDER));

        // File prefix: two spaces + type icon (icon also at char index 2).
        let file = glyph_prefix(false, false, "main.rs");
        let mut file_chars = file.chars();
        assert_eq!(file_chars.next(), Some(' '));
        assert_eq!(file_chars.next(), Some(' '));
        assert_eq!(file_chars.next(), Some(glyph::RUST));
    }
}
