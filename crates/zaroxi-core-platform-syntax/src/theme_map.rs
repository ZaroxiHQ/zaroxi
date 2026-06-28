//! Theme-aware mapping from Tree-sitter capture names to semantic token types.

use crate::highlight::Highlight;
use crate::theme_shim::{Color, SemanticColors};

/// A semantic token type that maps to a theme color.
///
/// These token types are used to convert highlight spans into theme-aware
/// styled spans for presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticTokenType {
    /// Keywords (control flow, declarations).
    Keyword,
    /// Function names and calls.
    Function,
    /// Method names (object-oriented calls).
    Method,
    /// String literals.
    String,
    /// Comments and documentation.
    Comment,
    /// Type and class names.
    Type,
    /// Variable identifiers.
    Variable,
    /// Constants and literals.
    Constant,
    /// Numeric literals.
    Number,
    /// Operators and similar tokens.
    Operator,
    /// Punctuation characters.
    Punctuation,
    /// Attributes, annotations.
    Attribute,
    /// Markup/HTML tags.
    Tag,
    /// Namespace or module identifiers.
    Namespace,
    /// Macro invocations.
    Macro,
    /// Property or field accessors.
    Property,
    /// Function parameters.
    Parameter,
    /// Built-in identifiers.
    Builtin,
    /// Escape sequences.
    Escape,
    /// Embedded language spans.
    Embedded,
    /// Regular expression tokens.
    Regex,
    /// Markdown heading tokens.
    MarkupHeading,
    /// Markdown list tokens.
    MarkupList,
    /// Markdown quote tokens.
    MarkupQuote,
    /// Markdown link tokens.
    MarkupLink,
    /// Markdown code tokens.
    MarkupCode,
    /// Markdown bold text tokens.
    MarkupBold,
    /// Markdown italic text tokens.
    MarkupItalic,
    /// Markdown strikethrough tokens.
    MarkupStrikethrough,
    /// Plain/unclassified text.
    Plain,
}

impl SemanticTokenType {
    /// Convert a Highlight classification into a semantic token type used for theming.
    ///
    /// This maps the small presenter-side highlight categories to the broader
    /// semantic token types consumed by theme resolution.
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
            Highlight::Punctuation => SemanticTokenType::Punctuation,
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

    /// Resolve this semantic token type to a concrete Color from the provided theme.
    ///
    /// Returns the theme color that corresponds to the token role.
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
///
/// Represents a contiguous region in the document annotated with a semantic
/// token type and the resolved theme color for presentation.
#[derive(Debug, Clone)]
pub struct StyledSpan {
    /// Start byte offset of the span.
    pub start: usize,
    /// End byte offset of the span (exclusive).
    pub end: usize,
    /// The semantic token type for this span.
    pub token_type: SemanticTokenType,
    /// Resolved color from the current theme.
    pub color: Color,
}

impl StyledSpan {
    /// Construct a StyledSpan from a HighlightSpan and the active SemanticColors.
    ///
    /// Converts the highlight classification into a semantic token type and
    /// resolves the color using the provided theme colors.
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
