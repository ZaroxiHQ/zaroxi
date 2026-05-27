// Focused command-bar helper implementations.
//
// Each function operates on `&mut super::DesktopComposition` or `&super::DesktopComposition`
// and mirrors the previous method bodies moved out of the large `desktop.rs` file.
// Exposed as `pub(crate)` to keep API surface minimal.

pub(crate) fn open_command_bar(comp: &mut super::DesktopComposition) {
    let mut labels: Vec<String> = Vec::new();
    labels.push("Refresh".to_string());
    labels.push("Open buffer".to_string());
    labels.push("Set active buffer".to_string());
    labels.push("Explain active buffer".to_string());
    labels.push("Request close active".to_string());
    labels.push("Confirm close: save".to_string());
    labels.push("Confirm close: discard".to_string());
    labels.push("Confirm close: cancel".to_string());

    comp.command_bar = Some(super::CommandBarState {
        open: true,
        commands: labels,
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
    if let Some(cb) = comp.command_bar.as_mut() {
        if !cb.commands.is_empty() {
            cb.selected = (cb.selected + 1) % cb.commands.len();
        }
    }
}

pub(crate) fn select_prev_command(comp: &mut super::DesktopComposition) {
    if let Some(cb) = comp.command_bar.as_mut() {
        if !cb.commands.is_empty() {
            if cb.selected == 0 {
                cb.selected = cb.commands.len() - 1;
            } else {
                cb.selected -= 1;
            }
        }
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
