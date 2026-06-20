//! Integration tests for path-based language detection (Phase 1 syntax
//! highlighting source of truth).

use std::path::Path;

use zaroxi_core_platform_syntax::language::LanguageId;

/// Language detection routes by extension to the correct canonical id.
/// Assertions use `as_str()` so they hold whether a given extension resolves
/// to a static variant or a runtime-registered `Dynamic` grammar (both map to
/// the same canonical id / query directory).
#[test]
fn from_path_routes_by_extension() {
    assert_eq!(LanguageId::from_path(Path::new("main.rs")).as_str(), "rust");
    assert_eq!(LanguageId::from_path(Path::new("Cargo.toml")).as_str(), "toml");
    assert_eq!(LanguageId::from_path(Path::new("README.md")).as_str(), "markdown");
    assert_eq!(LanguageId::from_path(Path::new("script.py")).as_str(), "python");
    assert_eq!(LanguageId::from_path(Path::new("app.js")).as_str(), "javascript");
}

#[test]
fn unknown_extension_falls_back_to_plain_text() {
    assert_eq!(LanguageId::from_path(Path::new("data.zzqq")), LanguageId::PlainText);
    assert_eq!(LanguageId::from_path(Path::new("no_extension")), LanguageId::PlainText);
}

#[test]
fn rust_and_dynamic_rust_share_canonical_id() {
    assert_eq!(LanguageId::Rust.as_str(), LanguageId::Dynamic("rust").as_str());
}
