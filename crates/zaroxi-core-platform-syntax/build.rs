//! Build script for `zaroxi-core-platform-syntax`.
//!
//! Tree-sitter grammar shared libraries are **committed** under
//! `runtime/treesitter/grammars/<os>-<arch>/` and loaded at runtime by the
//! `dynamic-loading` feature. This build script therefore does **not** compile
//! or download grammars.
//!
//! An earlier version shelled out to `cargo run --bin download_grammars` from
//! here, which is unreliable: it nests a `cargo` invocation inside the build
//! (risking lock contention / recursion) and needs network + a C toolchain at
//! build time. Grammar preparation is now an explicit, reusable step
//! (`tooling/scripts/prepare-treesitter.sh`) that both CI and developers run
//! before the syntax tests.
//!
//! If the runtime is missing for the current platform we emit an actionable
//! warning rather than attempting a fragile in-build install.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Capture the target triple so the `download_grammars` binary can drive the
    // `cc` crate correctly when it runs standalone (outside a build script,
    // where Cargo would otherwise provide TARGET/HOST). This is what makes
    // grammar compilation MSVC/clang/gcc-aware on every OS.
    if let Ok(target) = std::env::var("TARGET") {
        println!("cargo:rustc-env=ZAROXI_BUILD_TARGET={}", target);
    }

    let runtime_dir = get_runtime_dir();
    if !runtime_dir.exists() {
        println!(
            "cargo:warning=Tree-sitter runtime not found at {:?}. Syntax highlighting and its \
             integration tests require platform grammars. Prepare them with: \
             tooling/scripts/prepare-treesitter.sh",
            runtime_dir
        );
    }
}

/// Locate the canonical `runtime/treesitter` directory by walking up from this
/// crate's manifest directory (matches the runtime resolver in `src/runtime.rs`).
fn get_runtime_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut current = manifest_dir.clone();
    loop {
        let candidate = current.join("runtime/treesitter");
        if candidate.is_dir() {
            return candidate;
        }
        if !current.pop() {
            break;
        }
    }
    manifest_dir.join("runtime/treesitter")
}
