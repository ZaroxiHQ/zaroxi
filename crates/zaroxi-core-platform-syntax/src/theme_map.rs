//! Theme-aware mapping from Tree-sitter capture names to semantic token types.

use crate::highlight::Highlight;
use zaroxi_theme::colors::Color;
use zaroxi_theme::theme::SemanticColors;

/// A semantic token type that maps to a theme color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticTokenType {
    Keyword, Function, Method, String, Comment, Type, Variable, Constant,
    Number, Operator, Punctuation, Attribute, Tag, Namespace, Macro,
    Property, Parameter, Builtin, Escape, Embedded, Regex,
    MarkupHeading, MarkupList, MarkupQuote, MarkupLink, MarkupCode,
    MarkupBold, MarkupItalic, MarkupStrikethrough, Plain,
}

impl SemanticTokenType {
    pub fn from_highlight(highlight: Highlight) -> Self {
        match highlight {
            Highlight::Comment => SemanticTokenType::Comment,
            Highlight::String => SemanticTokenType::String,
            Highlight::Keyword => SemanticTokenType::Keyword,
            Highlight::Function => SemanticTokenType::Function,
            Highlight::Variable => SemanticTokenType::Variable,
            Highlight::Type => SemanticTokenType::Type,
            Highlight::Constant => SemanticTokenType::Constant,
            Highlight::Attribute => SemanticTokenType::Attribute,
            Highlight::Operator => SemanticTokenType::Operator,
            Highlight::Number => SemanticTokenType::Number,
            Highlight::Property => SemanticTokenType::Property,
            Highlight::Namespace => SemanticTokenType::Namespace,
            Highlight::Plain => SemanticTokenType::Plain,
        }
    }

    /// Get all available token types for debugging/configuration
    pub fn all_types() -> Vec<Self> {
        vec![
            SemanticTokenType::Keyword,
            SemanticTokenType::Function,
            SemanticTokenType::Method,
            SemanticTokenType::String,
            SemanticTokenType::Comment,
            SemanticTokenType::Type,
            SemanticTokenType::Variable,
            SemanticTokenType::Constant,
            SemanticTokenType::Number,
            SemanticTokenType::Operator,
            SemanticTokenType::Punctuation,
            SemanticTokenType::Attribute,
            SemanticTokenType::Tag,
            SemanticTokenType::Namespace,
            SemanticTokenType::Macro,
            SemanticTokenType::Property,
            SemanticTokenType::Parameter,
            SemanticTokenType::Builtin,
            SemanticTokenType::Escape,
            SemanticTokenType::Embedded,
            SemanticTokenType::Regex,
            SemanticTokenType::MarkupHeading,
            SemanticTokenType::MarkupList,
            SemanticTokenType::MarkupQuote,
            SemanticTokenType::MarkupLink,
            SemanticTokenType::MarkupCode,
            SemanticTokenType::MarkupBold,
            SemanticTokenType::MarkupItalic,
            SemanticTokenType::MarkupStrikethrough,
            SemanticTokenType::Plain,
        ]
    }

    pub fn theme_color(&self, colors: &SemanticColors) -> Color {
        match self {
            SemanticTokenType::Keyword => colors.syntax_keyword,
            SemanticTokenType::Function => colors.syntax_function,
            SemanticTokenType::Method => colors.syntax_method,
            SemanticTokenType::String => colors.syntax_string,
            SemanticTokenType::Comment => colors.syntax_comment,
            SemanticTokenType::Type => colors.syntax_type,
            SemanticTokenType::Variable => colors.syntax_variable,
            SemanticTokenType::Constant => colors.syntax_constant,
            SemanticTokenType::Number => colors.syntax_number,
            SemanticTokenType::Operator => colors.syntax_operator,
            SemanticTokenType::Punctuation => colors.syntax_punctuation,
            SemanticTokenType::Attribute => colors.syntax_attribute,
            SemanticTokenType::Tag => colors.syntax_tag,
            SemanticTokenType::Namespace => colors.syntax_namespace,
            SemanticTokenType::Macro => colors.syntax_macro,
            SemanticTokenType::Property => colors.syntax_property,
            SemanticTokenType::Parameter => colors.syntax_parameter,
            SemanticTokenType::Builtin => colors.syntax_builtin,
            SemanticTokenType::Escape => colors.syntax_escape,
            SemanticTokenType::Embedded => colors.syntax_embedded,
            SemanticTokenType::Regex => colors.syntax_regex,
            SemanticTokenType::MarkupHeading => colors.syntax_markup_heading,
            SemanticTokenType::MarkupList => colors.syntax_markup_list,
            SemanticTokenType::MarkupQuote => colors.syntax_markup_quote,
            SemanticTokenType::MarkupLink => colors.syntax_markup_link,
            SemanticTokenType::MarkupCode => colors.syntax_markup_code,
            SemanticTokenType::MarkupBold => colors.syntax_markup_bold,
            SemanticTokenType::MarkupItalic => colors.syntax_markup_italic,
            SemanticTokenType::MarkupStrikethrough => colors.syntax_markup_strikethrough,
            SemanticTokenType::Plain => colors.text_primary,
        }
    }
}

/// A styled span with its semantic token type and theme color.
#[derive(Debug, Clone)]
pub struct StyledSpan {
    pub start: usize,
    pub end: usize,
    pub token_type: SemanticTokenType,
    pub color: Color,
}

impl StyledSpan {
    pub fn from_highlight_span(
        span: &crate::highlight::HighlightSpan,
        colors: &SemanticColors,
    ) -> Self {
        let token_type = SemanticTokenType::from_highlight(span.highlight);
        let color = token_type.theme_color(colors);
        Self { start: span.start, end: span.end, token_type, color }
    }
}

/// Map a collection of highlight spans to styled spans using the given theme.
pub fn apply_theme(
    spans: &[crate::highlight::HighlightSpan],
    colors: &SemanticColors,
) -> Vec<StyledSpan> {
    spans.iter().map(|span| StyledSpan::from_highlight_span(span, colors)).collect()
}
