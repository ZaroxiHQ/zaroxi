#![allow(dead_code)]
//! Kernel math primitives for Zaroxi.
//!
//! This crate is intentionally small and zero-dependency.
//! It provides basic 2D geometry and color types used throughout higher layers.

pub mod color;
pub mod rect;
pub mod size;
pub mod vec2;

pub use color::Color;
pub use rect::Rect;
pub use size::Size;
pub use vec2::Vec2;
