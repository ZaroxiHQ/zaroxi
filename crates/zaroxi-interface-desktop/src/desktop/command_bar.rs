// Command-bar helpers operating on DesktopComposition.
// Pure policy (label list, index arithmetic) is delegated to
// zaroxi_application_workspace::workspace_view.

use zaroxi_application_workspace::workspace_view::{
    command_bar_labels, select_next_command_index, select_prev_command_index,
};

pub(crate) fn open_command_bar(comp: &mut super::DesktopComposition) {
    comp.command_bar = Some(super::CommandBarState {
        open: true,
        commands: command_bar_labels(),
        selected: 0,
        staged_arg: None,
    });
}

pub(crate) fn close_command_bar(comp: &mut super::DesktopComposition) {
    if let Some(cb) = comp.command_bar.as_mut() {
        cb.open = false;
    }
}

pub(crate) fn toggle_command_bar(comp: &mut super::DesktopComposition) {
    match comp.command_bar.as_mut() {
        Some(cb) => cb.open = !cb.open,
        None => open_command_bar(comp),
    }
}

pub(crate) fn is_command_bar_open(comp: &super::DesktopComposition) -> bool {
    comp.command_bar.as_ref().map(|c| c.open).unwrap_or(false)
}

pub(crate) fn latest_command_bar(
    comp: &super::DesktopComposition,
) -> Option<super::CommandBarState> {
    comp.command_bar.clone()
}

pub(crate) fn select_next_command(comp: &mut super::DesktopComposition) {
    if let Some(cb) = comp.command_bar.as_mut()
        && !cb.commands.is_empty()
    {
        cb.selected = select_next_command_index(cb.selected, cb.commands.len());
    }
}

pub(crate) fn select_prev_command(comp: &mut super::DesktopComposition) {
    if let Some(cb) = comp.command_bar.as_mut()
        && !cb.commands.is_empty()
    {
        cb.selected = select_prev_command_index(cb.selected, cb.commands.len());
    }
}

pub(crate) fn set_command_bar_staged_arg(
    comp: &mut super::DesktopComposition,
    arg: Option<String>,
) {
    if let Some(cb) = comp.command_bar.as_mut() {
        cb.staged_arg = arg;
    }
}
