//! `WidgetTree`: composes widgets and paints them in strict layer order.

use vello::Scene;
use zaroxi_interface_theme::CockpitTokens;

use crate::widget::{WidgetLayer, WidgetText, ZaroxiWidget};

/// A widget together with its computed `taffy::Layout` box.
pub struct PlacedWidget {
    /// The widget to paint.
    pub widget: Box<dyn ZaroxiWidget>,
    /// Its computed layout (location + size), produced by the taffy pass.
    pub layout: taffy::Layout,
}

/// Owns the cockpit's widgets and paints them into a single vello [`Scene`] in
/// the canonical layer order:
///
/// `background → editor → diff_layer → gutter → minimap → status_bar →
/// palette (when open) → tooltips`.
///
/// Within a layer, widgets paint in insertion order (stable).
#[derive(Default)]
pub struct WidgetTree {
    items: Vec<PlacedWidget>,
}

impl WidgetTree {
    /// Create an empty tree.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Add a widget with its computed layout.
    pub fn push(&mut self, widget: Box<dyn ZaroxiWidget>, layout: taffy::Layout) {
        self.items.push(PlacedWidget { widget, layout });
    }

    /// Number of widgets in the tree.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the tree has no widgets.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The canonical paint order (background → … → tooltip).
    pub fn paint_order() -> [WidgetLayer; 9] {
        WidgetLayer::ALL
    }

    /// Paint every widget into `scene` in ascending [`WidgetLayer`] order
    /// (stable by insertion order within a layer).
    pub fn paint(&self, scene: &mut Scene, theme: &CockpitTokens) {
        let mut order: Vec<usize> = (0..self.items.len()).collect();
        order.sort_by_key(|&i| (self.items[i].widget.layer(), i));
        for i in order {
            let item = &self.items[i];
            item.widget.paint(scene, &item.layout, theme);
        }
    }

    /// Collect every widget's positioned text runs (for the host to queue into
    /// the cosmic-text layer). Painted vector visuals and text are decoupled:
    /// [`WidgetTree::paint`] produces the vello scene; this produces the glyphs.
    pub fn collect_text(&self, theme: &CockpitTokens) -> Vec<WidgetText> {
        let mut out = Vec::new();
        for item in &self.items {
            out.extend(item.widget.text_items(&item.layout, theme));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Test widget that records the order in which it was painted.
    struct Recorder {
        layer: WidgetLayer,
        log: Rc<RefCell<Vec<WidgetLayer>>>,
    }

    impl ZaroxiWidget for Recorder {
        fn layer(&self) -> WidgetLayer {
            self.layer
        }
        fn paint(&self, _scene: &mut Scene, _layout: &taffy::Layout, _theme: &CockpitTokens) {
            self.log.borrow_mut().push(self.layer);
        }
    }

    #[test]
    fn paints_in_layer_order_regardless_of_insertion() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut tree = WidgetTree::new();
        // Insert deliberately out of order.
        for layer in [
            WidgetLayer::Palette,
            WidgetLayer::Background,
            WidgetLayer::StatusBar,
            WidgetLayer::Editor,
        ] {
            tree.push(Box::new(Recorder { layer, log: log.clone() }), taffy::Layout::default());
        }
        assert_eq!(tree.len(), 4);

        let mut scene = Scene::new();
        let theme = CockpitTokens::void();
        tree.paint(&mut scene, &theme);

        assert_eq!(
            *log.borrow(),
            vec![
                WidgetLayer::Background,
                WidgetLayer::Editor,
                WidgetLayer::StatusBar,
                WidgetLayer::Palette,
            ]
        );
    }
}
