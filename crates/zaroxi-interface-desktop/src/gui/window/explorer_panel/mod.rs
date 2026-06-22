/*!
Explorer panel module — owns panel-level view model, widget composition,
and action dispatch for the workspace file-tree sidebar.

Responsibilities:
- Build an `ExplorerPanelViewModel` from desktop composition state
- Produce structured `ExplorerPanelItem` rows for the widget builder
- Handle panel actions: toggle expand, open file, open workspace
- Keep rendering and interaction logic contained in this module
*/

mod actions;
pub mod icons;
mod view_model;

pub use actions::ExplorerPanelActions;
pub use view_model::ExplorerPanelViewModel;
