use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    #[error("language not supported: {0}")]
    LanguageNotSupported(String),
    #[error("grammar load error: {0}")]
    GrammarLoadError(String),
    #[error("query error: {0}")]
    QueryError(String),
    #[error("parse error")]
    ParseError,
    #[error("document not found")]
    DocumentNotFound,
    #[error("no syntax tree")]
    NoSyntaxTree,
    #[error("invalid edit range")]
    InvalidEditRange,
    #[error("parser error: {0}")]
    ParserError(String),
    #[error("metadata error: {0}")]
    MetadataError(String),
    #[error("unknown error: {0}")]
    Unknown(String),
}

/// Transport-friendly error shape for syntax errors.
///
/// This DTO is intentionally small and stable so infra-rpc can serialize it
/// into protocol::error types without leaking internal details.
#[derive(Debug, Serialize)]
pub struct SyntaxErrorDto {
    pub code: &'static str,
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
