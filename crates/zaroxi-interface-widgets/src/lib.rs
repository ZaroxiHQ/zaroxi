//! Zaroxi cockpit UI widgets.
//!
//! Interface-layer widget system for the Zaroxi "cockpit" UI: the
//! [`ZaroxiWidget`] trait, a layered [`WidgetTree`] composition, and the
//! individual cockpit components, composed as `vello` scenes and laid out with
//! `taffy`.
//!
//! Themes are **not** defined here. They are owned by `zaroxi-interface-theme`
//! (the source of truth); widgets consume [`CockpitTokens`] read-only. This
//! crate sits in the `interface` layer so it may depend on the theme crate
//! while honouring the `interface -> ... -> core` dependency direction.

pub mod tree;
pub mod widget;

pub use tree::{PlacedWidget, WidgetTree};
pub use widget::{
    WidgetLayer, ZaroxiWidget, brush, fill_rect, layout_rect, reduce_motion, set_reduce_motion,
};

/// The cockpit theme token set, re-exported from the theme crate (its owner).
pub use zaroxi_interface_theme::{CockpitTheme, CockpitTokens};
