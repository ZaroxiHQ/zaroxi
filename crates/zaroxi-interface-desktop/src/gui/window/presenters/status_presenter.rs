use zaroxi_core_engine_ui::ShellWorkContent;

use super::super::status_bar::StatusModel;

/// Shape the status bar view-model from live shell/editor state.
///
/// Thin presenter that delegates derivation to [`StatusModel::from_sources`],
/// keeping this layer consistent with the other shell presenters while the
/// real logic lives in the status bar module.
pub fn shape_status_content(
    work_content: &Option<ShellWorkContent>,
    workspace_name: Option<&str>,
    indent_sample: Option<&str>,
    cursor_line: usize,
    cursor_col: usize,
) -> StatusModel {
    StatusModel::from_sources(work_content, workspace_name, cursor_line, cursor_col, indent_sample)
}
