/*!
Workbench tab lifecycle for [`GuiApp`]: open/focus and close routed
through the canonical [`WorkbenchTabState`]. Kept isolated from
pointer-hit routing (see `navigation.rs`).
*/

use super::super::destination::{WorkbenchDestination, WorkbenchTabId};
use super::GuiApp;

impl GuiApp {
    /// Re-derive the activity-rail highlight from the canonical active tab.
    ///
    /// `rail_selected_index` is a pure UI reflection — NOT an authority.
    /// Called once per frame (and after every tab mutation) so the rail
    /// highlight can never disagree with `tab_state.active()`'s destination.
    /// Emits an env-gated `rail_reflect` diagnostic only when the reflection
    /// actually changes.
    pub(crate) fn sync_rail_reflection(&mut self) {
        let dest = self.tab_state.active().destination();
        let idx = dest.rail_index();
        if self.rail_selected_index != idx {
            if std::env::var("ZAROXI_DEBUG_TABS").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_TABS: rail_reflect selectedindex={idx} active_destination={dest:?}",
                );
            }
            self.rail_selected_index = idx;
        }
    }

    /// The single canonical activation entry point for a destination selected
    /// from the activity rail. Explorer focuses the file editor; every other
    /// destination opens/focuses its stable [`WorkbenchTabId::DestinationRoot`]
    /// tab through [`WorkbenchTabState`]. Reopening an existing destination
    /// focuses the existing tab (no duplicates).
    pub(crate) fn activate_destination(&mut self, dest: WorkbenchDestination) {
        let target = if dest.is_explorer() {
            WorkbenchTabId::Editor
        } else {
            WorkbenchTabId::DestinationRoot(dest)
        };
        self.open_or_focus_tab(target);
    }

    /// Open or focus a tab through the canonical tab state. Editor/Explorer
    /// focuses the file editor; non-file tabs are deduplicated and focused.
    /// Keeps the rail highlight in sync and triggers a cockpit rebuild + redraw.
    pub(crate) fn open_or_focus_tab(&mut self, id: WorkbenchTabId) {
        let old_active = self.tab_state.active().clone();
        self.tab_state.open_or_focus_non_file(id.clone());
        let new_active = self.tab_state.active().clone();
        if std::env::var("ZAROXI_DEBUG_TABS").as_deref() == Ok("1") {
            let already = old_active == new_active;
            eprintln!(
                "ZAROXI_TABS: tab_open id={id:?} kind=nonfile already_exists={already} focused=true",
            );
            eprintln!(
                "ZAROXI_TABS: tab_activate old={old_active:?} new={new_active:?} destination={:?} file_path=<none>",
                new_active.destination(),
            );
        }
        self.sync_rail_reflection();
        self.cockpit_status_fingerprint = 0;
        self.needs_render = true;
    }

    /// Close a tab by stable identity through the canonical tab state.
    /// Updates rail highlight and triggers cockpit rebuild + redraw.
    pub(crate) fn close_tab(&mut self, id: &WorkbenchTabId) {
        let _changed = self.tab_state.close_tab(id);
        if std::env::var("ZAROXI_DEBUG_TABS").as_deref() == Ok("1") {
            eprintln!("ZAROXI_TABS: tab_close id={id:?} next_active={:?}", self.tab_state.active(),);
        }
        self.sync_rail_reflection();
        self.cockpit_status_fingerprint = 0;
        self.needs_render = true;
    }
}
