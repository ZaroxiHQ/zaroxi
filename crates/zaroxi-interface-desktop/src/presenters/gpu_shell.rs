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

pub mod model;
pub mod paint;
pub mod transcript;

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
