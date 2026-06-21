use super::super::status_bar::{StatusInputs, StatusModel};

/// Shape the status bar view-model from gathered live inputs.
///
/// Thin presenter that delegates derivation to [`StatusModel::from_inputs`],
/// keeping this layer consistent with the other shell presenters while the real
/// logic lives in the status bar module. The app is responsible only for
/// gathering the raw [`StatusInputs`].
pub fn shape_status_content(inputs: &StatusInputs<'_>) -> StatusModel {
    StatusModel::from_inputs(inputs)
}
