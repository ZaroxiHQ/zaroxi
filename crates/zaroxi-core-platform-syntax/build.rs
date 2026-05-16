//! Build script for zaroxi-lang-syntax
//!
//! This build script downloads and compiles Tree-sitter grammars at compile time,
//! so they are available when the application runs.

use std::process::Command;

fn main() {
    // Always rerun if build script changes
    println!("cargo:rerun-if-changed=build.rs");

    // Check if runtime directory already exists
    let runtime_dir = get_runtime_dir();
    if runtime_dir.exists() {
        println!("cargo:warning=Runtime directory already exists at {:?}, skipping grammar installation", runtime_dir);
        return;
    }

    println!("cargo:warning=Installing Tree-sitter grammars at compile time...");

    // Run the download_grammars binary to install missing grammars
    let status = Command::new("cargo")
        .args(["run", "--bin", "download_grammars", "--", "install"])
        .status()
        .expect("Failed to run download_grammars binary");

    if !status.success() {
        panic!("Grammar installation failed");
    }

    println!("cargo:warning=Grammar installation complete");
}

fn get_runtime_dir() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
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
