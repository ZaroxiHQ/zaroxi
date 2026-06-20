//! Tests for tree-sitter runtime asset resolution.
//!
//! These validate the resolver logic itself (not just highlighting): the dev
//! workspace root resolves, grammar libraries exist for supported languages,
//! an explicit override takes precedence, and a missing root is explicit
//! (no silent fallback to the real bundled runtime).

use std::path::PathBuf;

use zaroxi_core_platform_syntax::runtime::{
    Runtime, resolve_runtime_root, resolve_runtime_root_with_override,
};

#[test]
fn resolves_runtime_root_in_dev_workspace() {
    let root = resolve_runtime_root().expect("runtime root should resolve in dev workspace");
    assert!(
        root.is_dir(),
        "resolved runtime root must be an existing directory: {}",
        root.display()
    );
    assert!(
        root.is_absolute(),
        "resolved runtime root must be canonical/absolute: {}",
        root.display()
    );
    // Both grammar and query trees derive from this root.
    assert!(root.join("grammars").is_dir() || root.join("languages").is_dir());
}

#[test]
fn grammar_libs_exist_for_supported_languages() {
    let runtime = Runtime::new();
    for lang in ["rust", "nix", "markdown", "toml"] {
        let lib = runtime.grammar_library_path(lang);
        assert!(
            lib.exists(),
            "grammar library for {} should exist at {} (runtime_root={})",
            lang,
            lib.display(),
            runtime.root().display(),
        );
    }
}

#[test]
fn explicit_override_takes_precedence() {
    // Use an explicit override directory (race-free; no process env mutation).
    let tmp = std::env::temp_dir().join(format!("zaroxi_rt_override_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();

    let resolved =
        resolve_runtime_root_with_override(Some(&tmp)).expect("override directory should resolve");
    let expected = std::fs::canonicalize(&tmp).unwrap();
    assert_eq!(resolved, expected, "explicit override must win over probes");

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn missing_root_is_explicit_not_silent() {
    // A runtime rooted at a non-existent directory must report missing assets
    // explicitly rather than silently resolving to the real bundled runtime.
    let bogus = PathBuf::from("/nonexistent/zaroxi/runtime/treesitter");
    let runtime = Runtime::with_root(bogus);
    assert!(!runtime.exists(), "bogus runtime root must not exist");
    assert!(
        !runtime.grammar_library_path("rust").exists(),
        "grammar lookups under a bogus root must fail (no silent fallback)"
    );
}
