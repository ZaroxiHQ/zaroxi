//! Zaroxi cockpit UI widgets.
//!
//! Interface-layer widget system for the Zaroxi "cockpit" UI: the
//! [`ZaroxiWidget`] trait, a layered [`WidgetTree`] composition, and the
//! individual cockpit components, composed as `vello` scenes and laid out with
//! `taffy`.
//!
//! Themes are **not** defined here. They are owned by `zaroxi-interface-theme`
//! (the source of truth); widgets consume [`SemanticColors`] read-only. This
//! crate sits in the `interface` layer so it may depend on the theme crate
//! while honouring the `interface -> ... -> core` dependency direction.

pub mod components;
pub mod tree;
pub mod welcome;
pub mod widget;

pub use components::{
    ActivityItem, ActivityRail, AiBand, AiMode, AiPredictionGutter, CockpitTab, CommandPalette,
    ContextBand, ContextCanvas, DestinationPlaceholder, ExtensionEntry, ExtensionsPanel,
    HealthBand, InstrumentStatus, LayoutBucket, LivingDiffLayer, LspStatus, MarkerKind, MetaChips,
    PaletteItem, PredictionCell, RelatedPanel, SemanticMinimap, SettingsPanel, SettingsRow,
    SettingsRowHit, SettingsRowKind, SettingsSection, StatusBar, StatusMarker, StatusMetrics,
    SymbolKind, TabLayoutResult, WORKBENCH_TAB_W, WorkbenchTabStrip, workbench_tab_layout,
};

pub use zaroxi_domain_settings::{FontPreference, Settings, SettingsAction, ThemePreference};

pub use tree::{PlacedWidget, WidgetTree};
pub use welcome::WelcomePanel;
pub use widget::{
    WidgetLayer, WidgetText, ZaroxiWidget, brush, color_arr, fill_rect, layout_rect, reduce_motion,
    set_reduce_motion,
};
