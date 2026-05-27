/*!
Facade module for the GPU-backed presenter.

This file is a small, stable re-export surface that preserves the original
public API while delegating responsibilities to small, focused sibling modules:

- model.rs       : region/slot/view models, GpuShellPresenter, tab model & actions, input mapping
- paint.rs       : paint-plan model + executor
- transcript.rs  : transcript formatting helpers

The goal is a same-crate modularization that keeps the external surface stable
and the internal responsibilities well-separated to avoid `gpu_shell.rs` becoming
a long-lived god file.
*/

// sibling modules `model`, `paint`, and `transcript` are declared at the
// presenters root (crates/zaroxi-interface-desktop/src/presenters/mod.rs)
// and are re-exported below. Avoid declaring them as nested modules here.

// Re-export the original public API surface so callers/tests remain unchanged.
pub use crate::presenters::model::{
    ContentActivity, FocusAction, GpuShellPresenter, GpuShellView, KeyEvent, Region, RegionKind,
    RegionView, ShellRegions, ShellTone, SlotName, SlotView, StatusEmphasis, TabAction, TabEntry,
    TabStrip, activate_focused, apply_focus_action, apply_tab_action, compute_focus_action_target,
    compute_tab_action_target, handle_focus_key_event, handle_key_event,
};
pub use crate::presenters::paint::{GpuPaintOp, GpuPaintPlan, GpuPaintRect, execute_paint_plan};
pub use crate::presenters::transcript::ShellRenderTranscript;

#[cfg(test)]
#[path = "gpu_shell_tests.rs"]
mod gpu_shell_tests;
