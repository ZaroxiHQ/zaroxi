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
    RegionKind, Region, ShellRegions, RegionView, SlotName, SlotView, ContentActivity, StatusEmphasis,
    ShellTone, GpuShellView, GpuShellPresenter, KeyEvent, handle_key_event, TabEntry, TabStrip,
    TabAction, compute_tab_action_target, apply_tab_action,
};
pub use crate::presenters::paint::{GpuPaintRect, GpuPaintOp, GpuPaintPlan, execute_paint_plan};
pub use crate::presenters::transcript::ShellRenderTranscript;

#[cfg(test)]
mod gpu_shell_tests;
```

scripts/run_presenter_checks.sh
```bash
<<<<<<< SEARCH
