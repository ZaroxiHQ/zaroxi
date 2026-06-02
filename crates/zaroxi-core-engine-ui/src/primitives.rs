use zaroxi_core_engine_style::InteractionState;
use zaroxi_kernel_math::Rect;

// ---------------------------------------------------------------------------
// Surface — the fundamental filled region with optional border and radius
// ---------------------------------------------------------------------------

/// A filled rectangular surface with optional border, corner radius, and
/// interaction state. This is the base building block for all shell chrome.
#[derive(Clone, Debug)]
pub struct Surface {
    pub rect: Rect,
    pub fill_color: [f32; 4],
    pub border_color: Option<[f32; 4]>,
    pub border_width: f32,
    pub corner_radius: f32,
    pub state: InteractionState,
}

impl Surface {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            fill_color: [0.0; 4],
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
            state: InteractionState::Normal,
        }
    }

    pub fn with_fill(mut self, color: impl Into<[f32; 4]>) -> Self {
        self.fill_color = color.into();
        self
    }

    pub fn with_border(mut self, color: [f32; 4], width: f32) -> Self {
        self.border_color = Some(color);
        self.border_width = width;
        self
    }

    pub fn with_radius(mut self, r: f32) -> Self {
        self.corner_radius = r;
        self
    }

    pub fn with_state(mut self, state: InteractionState) -> Self {
        self.state = state;
        self
    }
}

// ---------------------------------------------------------------------------
// HeaderBar — labeled header strip (panel title bar, tab strip header)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct HeaderBar {
    pub rect: Rect,
    pub label: String,
    pub fill_color: [f32; 4],
    pub text_color: [f32; 4],
    pub corner_radius_top: f32,
    pub state: InteractionState,
}

impl HeaderBar {
    pub fn new(rect: Rect, label: impl Into<String>) -> Self {
        Self {
            rect,
            label: label.into(),
            fill_color: [0.0; 4],
            text_color: [1.0; 4],
            corner_radius_top: 0.0,
            state: InteractionState::Normal,
        }
    }

    pub fn with_fill(mut self, color: impl Into<[f32; 4]>) -> Self {
        self.fill_color = color.into();
        self
    }

    pub fn with_text_color(mut self, color: impl Into<[f32; 4]>) -> Self {
        self.text_color = color.into();
        self
    }

    pub fn with_top_radius(mut self, r: f32) -> Self {
        self.corner_radius_top = r;
        self
    }
}

// ---------------------------------------------------------------------------
// PanelFrame — header bar + content surface grouped together
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct PanelFrame {
    pub rect: Rect,
    pub header: HeaderBar,
    pub content_fill: [f32; 4],
    pub border_color: Option<[f32; 4]>,
    pub border_width: f32,
    pub corner_radius: f32,
    pub state: InteractionState,
}

impl PanelFrame {
    pub fn new(rect: Rect, header_label: impl Into<String>, header_height: f32) -> Self {
        let header_rect = Rect::new(rect.x, rect.y, rect.width, header_height);
        let header = HeaderBar::new(header_rect, header_label);
        Self {
            rect,
            header,
            content_fill: [0.0; 4],
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
            state: InteractionState::Normal,
        }
    }

    pub fn content_rect(&self) -> Rect {
        let header_h = self.header.rect.height;
        Rect::new(
            self.rect.x,
            self.rect.y + header_h,
            self.rect.width,
            (self.rect.height - header_h).max(0.0),
        )
    }
}

// ---------------------------------------------------------------------------
// Divider — thin horizontal or vertical separator line
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Copy)]
pub enum DividerOrientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
pub struct Divider {
    pub rect: Rect,
    pub color: [f32; 4],
    pub orientation: DividerOrientation,
    pub subtle: bool,
}

impl Divider {
    pub fn horizontal(x: f32, y: f32, width: f32, color: [f32; 4]) -> Self {
        Self {
            rect: Rect::new(x, y, width, 1.0),
            color,
            orientation: DividerOrientation::Horizontal,
            subtle: false,
        }
    }

    pub fn vertical(x: f32, y: f32, height: f32, color: [f32; 4]) -> Self {
        Self {
            rect: Rect::new(x, y, 1.0, height),
            color,
            orientation: DividerOrientation::Vertical,
            subtle: false,
        }
    }

    pub fn subtle(mut self) -> Self {
        self.subtle = true;
        self
    }
}

// ---------------------------------------------------------------------------
// Inset — content padding helper, produces a reduced rect
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Copy)]
pub struct Inset {
    pub rect: Rect,
    pub pad_left: f32,
    pub pad_right: f32,
    pub pad_top: f32,
    pub pad_bottom: f32,
}

impl Inset {
    pub fn uniform(rect: Rect, pad: f32) -> Self {
        Self { rect, pad_left: pad, pad_right: pad, pad_top: pad, pad_bottom: pad }
    }

    pub fn from_rect(rect: Rect, left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self { rect, pad_left: left, pad_right: right, pad_top: top, pad_bottom: bottom }
    }

    pub fn inner_rect(&self) -> Rect {
        Rect::new(
            self.rect.x + self.pad_left,
            self.rect.y + self.pad_top,
            (self.rect.width - self.pad_left - self.pad_right).max(0.0),
            (self.rect.height - self.pad_top - self.pad_bottom).max(0.0),
        )
    }
}

// ---------------------------------------------------------------------------
// StatusPill — small rounded badge/indicator (status bar labels, badges)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct StatusPill {
    pub rect: Rect,
    pub label: String,
    pub fill_color: [f32; 4],
    pub text_color: [f32; 4],
}

impl StatusPill {
    pub fn new(rect: Rect, label: impl Into<String>) -> Self {
        Self { rect, label: label.into(), fill_color: [0.0; 4], text_color: [1.0; 4] }
    }

    pub fn with_fill(mut self, color: impl Into<[f32; 4]>) -> Self {
        self.fill_color = color.into();
        self
    }

    pub fn with_text_color(mut self, color: impl Into<[f32; 4]>) -> Self {
        self.text_color = color.into();
        self
    }
}

// ---------------------------------------------------------------------------
// TabChrome — single tab element in a tab strip
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TabChrome {
    pub rect: Rect,
    pub label: String,
    pub active: bool,
    pub fill_color: [f32; 4],
    pub text_color: [f32; 4],
    pub accent_strip: Option<[f32; 4]>,
    pub state: InteractionState,
}

impl TabChrome {
    pub fn new(rect: Rect, label: impl Into<String>) -> Self {
        Self {
            rect,
            label: label.into(),
            active: false,
            fill_color: [0.0; 4],
            text_color: [1.0; 4],
            accent_strip: None,
            state: InteractionState::Normal,
        }
    }

    pub fn active(mut self, accent: [f32; 4]) -> Self {
        self.active = true;
        self.accent_strip = Some(accent);
        self
    }

    pub fn with_fill(mut self, color: impl Into<[f32; 4]>) -> Self {
        self.fill_color = color.into();
        self
    }

    pub fn with_text_color(mut self, color: impl Into<[f32; 4]>) -> Self {
        self.text_color = color.into();
        self
    }
}

// ---------------------------------------------------------------------------
// IconSlot — small icon placeholder in rails/toolbars
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct IconSlot {
    pub rect: Rect,
    pub fill_color: [f32; 4],
    pub accent_indicator: Option<[f32; 4]>,
    pub state: InteractionState,
}

impl IconSlot {
    pub fn new(rect: Rect) -> Self {
        Self { rect, fill_color: [0.0; 4], accent_indicator: None, state: InteractionState::Normal }
    }

    pub fn with_fill(mut self, color: impl Into<[f32; 4]>) -> Self {
        self.fill_color = color.into();
        self
    }

    pub fn with_accent(mut self, color: [f32; 4]) -> Self {
        self.accent_indicator = Some(color);
        self
    }

    pub fn hovered(mut self) -> Self {
        self.state = InteractionState::Hover;
        self
    }

    pub fn active(mut self) -> Self {
        self.state = InteractionState::Active;
        self
    }
}

// ---------------------------------------------------------------------------
// ShellSurfaceSet — a complete shell described in engine primitives
// ---------------------------------------------------------------------------

/// Ordered collection of surfaces and chrome primitives that fully describe
/// a rendered shell frame. Layers are in paint order (background first).
#[derive(Clone, Debug)]
pub struct ShellSurfaceSet {
    pub surfaces: Vec<Surface>,
    pub headers: Vec<HeaderBar>,
    pub dividers: Vec<Divider>,
    pub status_pills: Vec<StatusPill>,
    pub tabs: Vec<TabChrome>,
    pub icons: Vec<IconSlot>,
}

impl ShellSurfaceSet {
    pub fn new() -> Self {
        Self {
            surfaces: Vec::new(),
            headers: Vec::new(),
            dividers: Vec::new(),
            status_pills: Vec::new(),
            tabs: Vec::new(),
            icons: Vec::new(),
        }
    }

    pub fn add_surface(&mut self, s: Surface) {
        self.surfaces.push(s);
    }

    pub fn add_header(&mut self, h: HeaderBar) {
        self.headers.push(h);
    }

    pub fn add_divider(&mut self, d: Divider) {
        self.dividers.push(d);
    }

    pub fn add_pill(&mut self, p: StatusPill) {
        self.status_pills.push(p);
    }

    pub fn add_tab(&mut self, t: TabChrome) {
        self.tabs.push(t);
    }

    pub fn add_icon(&mut self, i: IconSlot) {
        self.icons.push(i);
    }

    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
            && self.headers.is_empty()
            && self.dividers.is_empty()
            && self.status_pills.is_empty()
            && self.tabs.is_empty()
            && self.icons.is_empty()
    }

    /// Convert all primitives to flat rect primitives for the scene layer.
    pub fn to_rect_primitives(&self) -> Vec<zaroxi_core_engine_scene::RectPrimitive> {
        let mut rects = Vec::new();

        for s in &self.surfaces {
            rects.push(zaroxi_core_engine_scene::RectPrimitive::new(
                s.rect.x,
                s.rect.y,
                s.rect.width,
                s.rect.height,
                s.fill_color,
            ));
        }

        for h in &self.headers {
            rects.push(zaroxi_core_engine_scene::RectPrimitive::new(
                h.rect.x,
                h.rect.y,
                h.rect.width,
                h.rect.height,
                h.fill_color,
            ));
        }

        for d in &self.dividers {
            rects.push(zaroxi_core_engine_scene::RectPrimitive::new(
                d.rect.x,
                d.rect.y,
                d.rect.width,
                d.rect.height,
                d.color,
            ));
        }

        for p in &self.status_pills {
            rects.push(zaroxi_core_engine_scene::RectPrimitive::new(
                p.rect.x,
                p.rect.y,
                p.rect.width,
                p.rect.height,
                p.fill_color,
            ));
        }

        for t in &self.tabs {
            rects.push(zaroxi_core_engine_scene::RectPrimitive::new(
                t.rect.x,
                t.rect.y,
                t.rect.width,
                t.rect.height,
                t.fill_color,
            ));
        }

        for i in &self.icons {
            rects.push(zaroxi_core_engine_scene::RectPrimitive::new(
                i.rect.x,
                i.rect.y,
                i.rect.width,
                i.rect.height,
                i.fill_color,
            ));
        }

        rects
    }
}

impl Default for ShellSurfaceSet {
    fn default() -> Self {
        Self::new()
    }
}
