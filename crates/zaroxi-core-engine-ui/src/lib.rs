pub mod bar;
pub mod composer;
pub mod content;
pub mod interaction;
pub mod layout;
pub mod primitives;
pub mod shell_builder;
pub mod syntax_tokenizer;
pub mod ui;
pub mod widgets;
pub mod work_content;

pub use bar::Bar;
pub use composer::{compose_bar_labels, compose_bars, compose_bars_scene, compose_content_view};
pub use content::ContentView;
pub use interaction::{PointerButton, WidgetAction, WidgetInteractionModel};
pub use primitives::{
    Divider, DividerOrientation, HeaderBar, IconSlot, Inset, PanelFrame, ShellSurfaceSet,
    StatusPill, Surface, TabChrome,
};
pub use shell_builder::{build_shell_surface_set, build_shell_widget_tree};
pub use widgets::{PanelHeaderAction, ShellWidget, ShellWidgetTree};
pub use work_content::{HighlightKind, LineHighlight, ShellWorkContent, SyntaxHighlights};
pub use zaroxi_core_engine_layout::{ShellLayout, build_shell_ui};
pub use zaroxi_core_engine_scene::{LabelPrimitive, RectPrimitive, WidgetScene};
pub use zaroxi_core_engine_style::{
    InteractionState, PanelRole, StyleTokens, SurfaceRole, ThemeColor, WidgetId,
};
