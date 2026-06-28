/*!
Workbench tab lifecycle for [`GuiApp`]: open/focus and close routed
through the canonical [`WorkbenchTabState`]. Kept isolated from
pointer-hit routing (see `navigation.rs`).
*/

use super::GuiApp;

impl GuiApp {
    /// Open or focus a tab through the canonical tab state. Editor/Explorer
    /// focuses the file editor; non-file tabs are deduplicated and focused.
    /// Keeps the rail highlight in sync and triggers a cockpit rebuild + redraw.
    pub(crate) fn open_or_focus_tab(&mut self, id: super::super::destination::WorkbenchTabId) {
        self.tab_state.open_or_focus_non_file(id);
        self.rail_selected_index = self.tab_state.active().destination().rail_index();
        self.cockpit_status_fingerprint = 0;
        self.needs_render = true;
    }

    /// Close a tab by stable identity through the canonical tab state.
    /// Updates rail highlight and triggers cockpit rebuild + redraw.
    pub(crate) fn close_tab(&mut self, id: &super::super::destination::WorkbenchTabId) {
        let _changed = self.tab_state.close_tab(id);
        self.rail_selected_index = self.tab_state.active().destination().rail_index();
        self.cockpit_status_fingerprint = 0;
        self.needs_render = true;
    }
}
