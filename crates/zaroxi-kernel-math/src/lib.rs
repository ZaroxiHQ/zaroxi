#![allow(dead_code)]
//! Kernel math primitives for Zaroxi.
//!
//! This crate is intentionally small and zero-dependency.
//! It provides basic 2D geometry and color types used throughout higher layers.

pub mod vec2;
pub mod size;
pub mod rect;
pub mod color;

pub use vec2::Vec2;
pub use size::Size;
pub use rect::Rect;
pub use color::Color;
