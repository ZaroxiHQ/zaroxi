#![allow(dead_code)]
//! Minimal shared error type for Zaroxi kernel layer.

use std::fmt;

/// Result type alias used across kernel crates.
pub type ZResult<T> = Result<T, ZaroxiError>;

/// Minimal cross-crate error enum for early phases.
#[derive(Debug)]
pub enum ZaroxiError {
    Io(std::io::Error),
    Render(String),
    Parse(String),
    Network(String),
    InvalidState(String),
}

impl fmt::Display for ZaroxiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZaroxiError::Io(e) => write!(f, "IO error: {}", e),
            ZaroxiError::Render(s) => write!(f, "Render error: {}", s),
            ZaroxiError::Parse(s) => write!(f, "Parse error: {}", s),
            ZaroxiError::Network(s) => write!(f, "Network error: {}", s),
            ZaroxiError::InvalidState(s) => write!(f, "Invalid state: {}", s),
        }
    }
}

impl std::error::Error for ZaroxiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ZaroxiError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ZaroxiError {
    fn from(e: std::io::Error) -> Self {
        ZaroxiError::Io(e)
    }
}
