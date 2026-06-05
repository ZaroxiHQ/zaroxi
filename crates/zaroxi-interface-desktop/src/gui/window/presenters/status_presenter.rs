use zaroxi_core_engine_ui::ShellWorkContent;

use super::super::status_bar::StatusBarData;

/// Shape status bar text from work_content and live cursor state.
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

    StatusBarData {
        status_line: cursor_line,
        status_col: cursor_col,
        status_language: status_language.to_string(),
    }
}
