// Small internal state/helpers for the desktop composition module.
//
// This file holds a tiny helper moved out of the top-level `desktop.rs` to keep
// the facade compact. The helper is re-exported as `pub(crate)` from the parent
// module so existing callers in sibling submodules (e.g. `composition`) continue
// to call `super::command_kind_short_name(...)` without any changes.

/// Convert CommandKind to a short stable name used in tiny shell-facing status lines.
#[allow(dead_code)]
pub(crate) fn command_kind_short_name(kind: &crate::ports::CommandKind) -> &'static str {
    match kind {
        crate::ports::CommandKind::BootWorkspace { .. } => "BootWorkspace",
        crate::ports::CommandKind::OpenBuffer { .. } => "OpenBuffer",
        crate::ports::CommandKind::UpdateBuffer { .. } => "UpdateBuffer",
        crate::ports::CommandKind::SetActiveBuffer { .. } => "SetActiveBuffer",
        crate::ports::CommandKind::ExplainActiveBuffer => "ExplainActiveBuffer",
        crate::ports::CommandKind::DispatchAppCommand { .. } => "DispatchAppCommand",
    }
}
