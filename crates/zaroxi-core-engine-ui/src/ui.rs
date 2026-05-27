/// Minimal UI node/style shapes reserved for future extension.
/// For Phase 3 we keep these tiny and engine-internal; the composer builds
/// the concrete shell directly.
#[derive(Clone, Debug)]
pub enum Direction {
    Row,
    Column,
}

#[derive(Clone, Debug)]
pub struct UiStyle {
    pub direction: Direction,
    pub padding: f32,
    pub gap: f32,
}

impl Default for UiStyle {
    fn default() -> Self {
        UiStyle { direction: Direction::Column, padding: 0.0, gap: 0.0 }
    }
}
