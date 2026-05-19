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
use crate::gpu_shell_adapter::{view_model_to_regions, view_model_to_regions_from_scratch};
use crate::presenters::gpu_shell::ShellRegions;
use crate::actions::{refresh_and_get_shell_context, set_active_buffer_and_get_shell_context};

/// Apply the given action using the existing action/refresh helpers and return
/// adapted ShellRegions for the presenter. This function avoids duplicating
/// action logic in the binary and exercises the real refresh path.
pub fn apply_action_and_get_regions(action: Action, width: u32, height: u32) -> ShellRegions {
    match action {
        Action::SetActiveBuffer(name) => {
            // Try to use the application-facing helper that sets the active buffer
            // and returns a refreshed shell-facing model. Fall back to a scratch
            // model if anything is not available at runtime.
            //
            // We intentionally call the existing helper so the application layer
            // is exercised (no duplicated action logic in the binary).
            let model = set_active_buffer_and_get_shell_context(name);
            view_model_to_regions(&model, width, height)
        }
        // For other actions we perform a general refresh and adapt the result.
        _ => {
            let model = refresh_and_get_shell_context();
            view_model_to_regions(&model, width, height)
        }
    }
}
