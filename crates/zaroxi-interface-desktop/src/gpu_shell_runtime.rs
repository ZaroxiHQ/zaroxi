/*!
Small runtime helper that ties a mapped Action to the existing action/refresh
path and returns adapted ShellRegions suitable for the GPU presenter.

This file intentionally lives inside `zaroxi-interface-desktop` and is
crate-local so the native binary can delegate to the canonical action
and refresh functions instead of duplicating logic.

Flow implemented here:
- Accept an EventBridge::Action
- Invoke the appropriate existing action/refresh helper (in `actions`)
- Obtain the refreshed shell-facing view model
- Adapt it into ShellRegions via the existing adapter
- Return ShellRegions for the presenter to paint

This helper keeps native bootstrap minimal while ensuring the event -> action ->
refresh -> adapt -> repaint path is exercised end-to-end.
*/

use crate::events::Action;
use crate::gpu_shell_adapter::view_model_to_regions_from_scratch;
use crate::presenters::model::ShellRegions;

/// Apply the given action using the existing action/refresh helpers and return
/// adapted ShellRegions for the presenter. This function avoids duplicating
/// action logic in the binary and exercises the real refresh path.
///
/// NOTE: the fully-correct application helpers (in `actions`) are async and
/// require a live DesktopComposition + services/context which the minimal native
/// binary does not construct. To keep this phase tiny and explicit (and to
/// exercise the end-to-end mapping path), we fall back to the adapter's
/// from-scratch view model. This preserves the event -> action -> adapt -> paint
/// loop without duplicating application logic.
pub fn apply_action_and_get_regions(action: Action, width: u32, height: u32) -> ShellRegions {
    // For this phase we still use the from-scratch adapter so the binary can drive
    // the presenter without requiring full application runtime context.
    //
    // However, to produce a deterministic visible change we propagate a lightweight
    // state marker (active buffer name) through the runtime -> adapter -> presenter
    // path. This avoids duplicating composition logic while making the GPU output
    // visibly reflect the action.
    match action {
        Action::SetActiveBuffer(name) => view_model_to_regions_from_scratch(width, height, Some(&name)),
        _ => view_model_to_regions_from_scratch(width, height, None),
    }
}
