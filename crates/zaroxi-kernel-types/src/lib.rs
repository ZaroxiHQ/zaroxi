#![deny(missing_docs)]
//! Kernel shared plain-value types.
//!
//! Minimal, stable value types permitted at the kernel layer. These are pure
//! data containers only (no IO, no runtime, no platform).

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Strongly-typed identifier backed by a UUID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id(Uuid);

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl Id {
    /// Create a new random Id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Construct from an existing Uuid.
    pub fn from_uuid(u: Uuid) -> Self {
        Self(u)
    }

    /// Return the inner Uuid.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Position in a text-like sequence (line/column).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Position {
    /// 0-based line index.
    pub line: u32,
    /// 0-based column index (in characters).
    pub column: u32,
}

impl Position {
    /// Create a zero position.
    pub fn zero() -> Self {
        Self { line: 0, column: 0 }
    }
}

/// Inclusive span between two positions.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Span {
    /// Start position (inclusive).
    pub start: Position,
    /// End position (exclusive).
    pub end: Position,
}

impl Span {
    /// Create a span from start to end.
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Is the span empty?
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// Programming language identifier for editor/workspace usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// TypeScript with JSX (files ending in .tsx)
    TypeScriptJsx,
    /// TypeScript (files ending in .ts)
    TypeScript,
    /// JavaScript (files ending in .js/.mjs/.cjs)
    JavaScript,
    /// JavaScript configuration-style files (e.g. config.js)
    JavaScriptConfig,
    /// JSON files (.json)
    Json,
    /// Rust source files (.rs)
    Rust,
    /// TOML files (.toml)
    Toml,
    /// Unknown or plain text files
    Unknown,
}

impl Language {
    /// Map a file extension (without dot) to a language.
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "tsx" => Language::TypeScriptJsx,
            "ts" => Language::TypeScript,
            "jsx" => Language::JavaScript,
            "js" | "mjs" | "cjs" => Language::JavaScript,
            "json" => Language::Json,
            "rs" => Language::Rust,
            "toml" => Language::Toml,
            _ => Language::Unknown,
        }
    }

    /// Human-friendly display name.
    pub fn display_name(self) -> &'static str {
        match self {
            Language::TypeScriptJsx => "TypeScript JSX",
            Language::TypeScript => "TypeScript",
            Language::JavaScript => "JavaScript",
            Language::JavaScriptConfig => "JavaScript Config",
            Language::Json => "JSON",
            Language::Rust => "Rust",
            Language::Toml => "TOML",
            Language::Unknown => "Plain Text",
        }
    }

    /// Short label used for small UI badges.
    pub fn icon_label(self) -> &'static str {
        match self {
            Language::TypeScriptJsx => "TS",
            Language::TypeScript => "TS",
            Language::JavaScript => "JS",
            Language::JavaScriptConfig => "JS",
            Language::Json => "{}",
            Language::Rust => "RS",
            Language::Toml => "TL",
            Language::Unknown => "TXT",
        }
    }
}

/// Line ending representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    /// LF ("\n")
    Lf,
    /// CRLF ("\r\n")
    CrLf,
    /// CR ("\r")
    Cr,
}

impl LineEnding {
    /// Short display string.
    pub fn display(self) -> &'static str {
        match self {
            LineEnding::Lf => "LF",
            LineEnding::CrLf => "CRLF",
            LineEnding::Cr => "CR",
        }
    }
}

/// Text encoding representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// UTF-8 encoding
    Utf8,
    /// UTF-16 Little Endian
    Utf16Le,
    /// UTF-16 Big Endian
    Utf16Be,
}

impl Encoding {
    /// Display name for the encoding.
    pub fn display(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf16Le => "UTF-16 LE",
            Encoding::Utf16Be => "UTF-16 BE",
        }
    }
}

/// Indentation style for editors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentStyle {
    /// Use tab characters for indentation.
    Tabs,
    /// Use spaces for indentation, parameter is the number of spaces.
    Spaces(u8),
}

impl IndentStyle {
    /// User-facing display string.
    pub fn display(self) -> String {
        match self {
            IndentStyle::Tabs => "Tabs".to_string(),
            IndentStyle::Spaces(n) => format!("Spaces {}", n),
        }
    }
}
