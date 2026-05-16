#![allow(dead_code)]
//! Minimal placeholder for text subsystem.
//!
//! This crate intentionally exposes a tiny, inert API suitable as the
//! future integration point for "cosmic-text" or a custom text pipeline.

/// Represents a text subsystem handle. Currently a stub.
pub struct TextEngine {
    pub dpi_scale: f32,
}

impl TextEngine {
    /// Create a new stub text engine.
    pub fn new(dpi_scale: f32) -> Self {
        Self { dpi_scale }
    }
}
 #![allow(dead_code)]
 //! Minimal placeholder for text subsystem.
 //!
 //! This crate intentionally exposes a tiny, inert API suitable as the
 //! future integration point for "cosmic-text" or a custom text pipeline.

 /// Represents a text subsystem handle. Currently a stub.
 pub struct TextEngine {
     pub dpi_scale: f32,
 }

 impl TextEngine {
     /// Create a new stub text engine.
     pub fn new(dpi_scale: f32) -> Self {
         Self { dpi_scale }
     }
 }
