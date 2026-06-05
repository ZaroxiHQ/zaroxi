//! Platform clipboard integration for the Zaroxi engine.
//!
//! Wraps `arboard` to provide a synchronous clipboard interface for
//! copy/paste operations from interface layer code.
//!
//! Phase 64: implemented copy_text() / get_text() for editor clipboard.

use std::error::Error;

/// Copy the given text to the system clipboard.
pub fn copy_text(text: &str) -> Result<(), Box<dyn Error>> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}

/// Get text from the system clipboard.
pub fn get_text() -> Result<String, Box<dyn Error>> {
    let mut clipboard = arboard::Clipboard::new()?;
    let text = clipboard.get_text()?;
    Ok(text)
}
