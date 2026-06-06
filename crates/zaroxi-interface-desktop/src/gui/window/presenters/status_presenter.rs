use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_core_engine_ui::chrome::StatusBarZones;

use super::super::status_bar::StatusBarData;

/// Shape status bar content from work_content and live cursor state.
/// Returns structured left/right zones for chrome-aware formatting.
pub fn shape_status_content(
    work_content: &Option<ShellWorkContent>,
    cursor_line: usize,
    cursor_col: usize,
) -> StatusBarData {
    let wc = match work_content {
        Some(w) => w,
        None => return StatusBarData::default(),
    };

    let status_language = wc
        .active_file
        .as_ref()
        .and_then(|f| f.rsplit('.').next())
        .map(|ext| match ext {
            "rs" => "Rust",
            "toml" => "TOML",
            "md" => "Markdown",
            "json" => "JSON",
            "py" => "Python",
            "js" => "JavaScript",
            "ts" => "TypeScript",
            _ => ext,
        })
        .unwrap_or("No file");
    let status_language = status_language.to_string();

    let left_segments = vec![
        "Ready".to_string(),
        format!("Ln {}, Col {}", cursor_line + 1, cursor_col + 1),
        "UTF-8".to_string(),
        "LF".to_string(),
    ];
    let right_segments = vec![status_language];

    StatusBarData {
        status_line: cursor_line,
        status_col: cursor_col,
        status_language: String::new(), // unused — kept for compat
        status_zones: Some(StatusBarZones { left_segments, right_segments }),
    }
}
