#![allow(dead_code)]
#![allow(unused_imports)]
// Gutter crate: defines a minimal gutter model and helpers to reserve gutter
// space for editor content. The implementation intentionally keeps the model
// tiny so presenters can extend visuals without changing core math.

mod gutter;
pub use gutter::GutterModel;

#[cfg(test)]
mod tests {
    use super::GutterModel;
    #[test]
    fn gutter_basic() {
        let g = GutterModel::new(56);
        assert_eq!(g.content_inset(), 56);
    }
}
