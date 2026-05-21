use serde::Serialize;

/// Errors produced by the syntax subsystem.
///
/// High-level, caller-facing errors that consumers can match on to handle
/// syntax parsing, query, or runtime failures.
#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    /// Language requested is not supported by the runtime/registry.
    #[error("language not supported: {0}")]
    LanguageNotSupported(String),

    /// Loading or compiling a grammar failed.
    #[error("grammar load error: {0}")]
    GrammarLoadError(String),

    /// Query execution error (Tree-sitter query).
    #[error("query error: {0}")]
    QueryError(String),

    /// Generic parse failure.
    #[error("parse error")]
    ParseError,

    /// Document not found in the manager.
    #[error("document not found")]
    DocumentNotFound,

    /// No syntax tree is available for the requested document.
    #[error("no syntax tree")]
    NoSyntaxTree,

    /// An edit range supplied by the caller was invalid.
    #[error("invalid edit range")]
    InvalidEditRange,

    /// Underlying parser error with message.
    #[error("parser error: {0}")]
    ParserError(String),

    /// Metadata failure (e.g. missing language metadata).
    #[error("metadata error: {0}")]
    MetadataError(String),

    /// Unknown or unexpected error.
    #[error("unknown error: {0}")]
    Unknown(String),
}

/// Transport-friendly error shape for syntax errors.
///
/// Small DTO intended for serialization across IPC/HTTP boundaries.
#[derive(Debug, Serialize)]
pub struct SyntaxErrorDto {
    /// Short machine-readable error code.
    pub code: &'static str,
    /// Human-readable error message.
    pub message: String,
}

impl From<&SyntaxError> for SyntaxErrorDto {
    fn from(e: &SyntaxError) -> Self {
        let code = match e {
            SyntaxError::LanguageNotSupported(_) => "LANGUAGE_NOT_SUPPORTED",
            SyntaxError::GrammarLoadError(_) => "GRAMMAR_LOAD_ERROR",
            SyntaxError::QueryError(_) => "QUERY_ERROR",
            SyntaxError::ParseError => "PARSE_ERROR",
            SyntaxError::DocumentNotFound => "DOCUMENT_NOT_FOUND",
            SyntaxError::NoSyntaxTree => "NO_SYNTAX_TREE",
            SyntaxError::InvalidEditRange => "INVALID_EDIT_RANGE",
            SyntaxError::ParserError(_) => "PARSER_ERROR",
            SyntaxError::MetadataError(_) => "METADATA_ERROR",
            SyntaxError::Unknown(_) => "UNKNOWN_ERROR",
        };
        SyntaxErrorDto {
            code,
            message: e.to_string(),
        }
    }
}
