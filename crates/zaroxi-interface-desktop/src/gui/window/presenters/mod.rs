mod ai_presenter;
mod editor_presenter;
mod explorer_presenter;
mod status_presenter;

pub use ai_presenter::shape_ai_content;
pub use editor_presenter::{
    shape_editor_content, shape_editor_content_incremental, shape_editor_content_plain,
};
pub use explorer_presenter::shape_explorer_content;
pub use status_presenter::shape_status_content;
