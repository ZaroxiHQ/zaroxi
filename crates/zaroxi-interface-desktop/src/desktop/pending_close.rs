// Small pending-close helpers moved out of the main desktop module.
// These are intentionally trivial wrappers around the composition's field
// to keep the DesktopComposition impl compact and focused.

pub(crate) fn set_pending_close(comp: &mut super::DesktopComposition, pending: super::PendingClose) {
    comp.pending_close = Some(pending);
}

pub(crate) fn clear_pending_close(comp: &mut super::DesktopComposition) {
    comp.pending_close = None;
}

pub(crate) fn has_pending_close(comp: &super::DesktopComposition) -> bool {
    comp.pending_close.is_some()
}

pub(crate) fn latest_pending_close(comp: &super::DesktopComposition) -> Option<super::PendingClose> {
    comp.pending_close.clone()
}
