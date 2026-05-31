/*!
Reusable engine-level Bar widget.

A `Bar` is a labeled rectangular region in the shell. It is intentionally
generic: no desktop-specific concepts (status bar, toolbar, titlebar) are
baked in. Higher layers adapt application state into `Bar` instances and
compose them into the shell.

Design:
- Owns a label (short single-line display string) and a kernel-math Rect.
- No rendering, no colors, no fonts — purely structural.
- Consumer crates (interface-desktop, renderers) convert `Bar` into their
  own drawing/transcript primitives.
*/

use zaroxi_kernel_math::Rect;

#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    pub label: String,
    pub rect: Rect,
}

impl Bar {
    pub fn new(label: impl Into<String>, rect: Rect) -> Self {
        Self { label: label.into(), rect }
    }
}
