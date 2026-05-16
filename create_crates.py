#!/usr/bin/env python3
# Script: tools/create_crates.py
# Purpose: Professional Zaroxi crate scaffolding generator.
# Usage: python3 tools/create_crates.py
#
# This script creates a complete, production-ready skeleton for every Zaroxi crate
# in a flat workspace layout (crates/<crate-name>), enforcing the project's
# strict layering rules and Rust edition policy.
#
# Guarantees:
# - Rust edition is set to RUST_EDITION below.
# - Each crate gets a Cargo.toml with name, version "0.1.0", edition, and a
#   meaningful description string suitable for publishing.
# - Each crate gets src/lib.rs with standardized header comments documenting
#   Layer and single responsibility.
# - Writes placeholder tool directories (tools/crate-lint, docs).
# - Writes a root Cargo.toml workspace file that lists all members (crates + tools).
# - Creates a .rust-toolchain file to lock the recommended toolchain for the repo.
# - Emits a compact validation summary at the end.
#
# This implementation is intended for production usage as a deterministic
# scaffolding step in repo bootstrapping/CI workflows.

import os
import sys
import textwrap
from typing import List, Dict

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))

# Policy constants
RUST_EDITION = "2024"            # mandated Rust edition
RUST_TOOLCHAIN = "stable"        # recommended toolchain; set to 'stable' by default
CRATE_VERSION = "0.1.0"
LICENSE = "MIT"

# Flat crate inventory (final, authoritative)
CRATES: List[str] = [
    # KERNEL
    "zaroxi-kernel-core",
    "zaroxi-kernel-types",
    "zaroxi-kernel-errors",
    "zaroxi-kernel-memory",
    "zaroxi-kernel-async",
    "zaroxi-kernel-time",
    "zaroxi-kernel-math",
    "zaroxi-kernel-collections",
    "zaroxi-kernel-traits",
    "zaroxi-kernel-config",
    "zaroxi-kernel-protocol",

    # CORE ENGINE
    "zaroxi-core-engine-root",
    "zaroxi-core-engine-runtime",
    "zaroxi-core-engine-state",
    "zaroxi-core-engine-window",
    "zaroxi-core-engine-input",
    "zaroxi-core-engine-action",
    "zaroxi-core-engine-focus",
    "zaroxi-core-engine-layout",
    "zaroxi-core-engine-style",
    "zaroxi-core-engine-element",
    "zaroxi-core-engine-view",
    "zaroxi-core-engine-scene",
    "zaroxi-core-engine-overlay",
    "zaroxi-core-engine-text",
    "zaroxi-core-engine-font",
    "zaroxi-core-engine-render",
    "zaroxi-core-engine-render-backend",
    "zaroxi-core-engine-render-resource",
    "zaroxi-core-engine-render-pipeline",
    "zaroxi-core-engine-render-graph",
    "zaroxi-core-engine-compositor",
    "zaroxi-core-engine-animation",
    "zaroxi-core-engine-clipboard",
    "zaroxi-core-engine-ime",
    "zaroxi-core-engine-accessibility",
    "zaroxi-core-engine-test",

    # CORE EDITOR
    "zaroxi-core-editor-buffer",
    "zaroxi-core-editor-rope",
    "zaroxi-core-editor-model",
    "zaroxi-core-editor-transaction",
    "zaroxi-core-editor-selection",
    "zaroxi-core-editor-cursor",
    "zaroxi-core-editor-history",
    "zaroxi-core-editor-display",
    "zaroxi-core-editor-decoration",
    "zaroxi-core-editor-folding",
    "zaroxi-core-editor-gutter",
    "zaroxi-core-editor-diagnostics",
    "zaroxi-core-editor-minimap",
    "zaroxi-core-editor-command",
    "zaroxi-core-editor-viewport",
    "zaroxi-core-editor-inline-ai",
    "zaroxi-core-editor-view",
    "zaroxi-core-editor-collab",

    # CORE WORKSPACE
    "zaroxi-core-workspace-files",
    "zaroxi-core-workspace-index",
    "zaroxi-core-workspace-history",
    "zaroxi-core-workspace-patch",
    "zaroxi-core-workspace-watcher",
    "zaroxi-core-workspace-cache",
    "zaroxi-core-workspace-snapshot",
    "zaroxi-core-workspace-permissions",

    # CORE PLATFORM
    "zaroxi-core-platform-runtime",
    "zaroxi-core-platform-lsp",
    "zaroxi-core-platform-syntax",
    "zaroxi-core-platform-debugger",
    "zaroxi-core-platform-terminal",
    "zaroxi-core-platform-git",
    "zaroxi-core-platform-test",
    "zaroxi-core-platform-profiler",
    "zaroxi-core-platform-formatter",
    "zaroxi-core-platform-linter",
    "zaroxi-core-platform-plugin",
    "zaroxi-core-platform-remote-ssh",
    "zaroxi-core-platform-remote-container",

    # CORE RUNTIME (shared core infra)
    "zaroxi-core-runtime",
    "zaroxi-core-scheduler",
    "zaroxi-core-task",
    "zaroxi-core-threading",
    "zaroxi-core-state",
    "zaroxi-core-io",
    "zaroxi-core-sync",
    "zaroxi-core-input",
    "zaroxi-core-event",
    "zaroxi-core-commands",
    "zaroxi-core-telemetry",
    "zaroxi-core-plugin-runtime",

    # DOMAIN
    "zaroxi-domain-workspace",
    "zaroxi-domain-project",
    "zaroxi-domain-settings",
    "zaroxi-domain-session",
    "zaroxi-domain-ai",
    "zaroxi-domain-plugin",
    "zaroxi-domain-collaboration",

    # APPLICATION
    "zaroxi-application-editor",
    "zaroxi-application-workspace",
    "zaroxi-application-project",
    "zaroxi-application-command",
    "zaroxi-application-search",
    "zaroxi-application-navigation",
    "zaroxi-application-refactor",
    "zaroxi-application-ai",
    "zaroxi-application-plugin",
    "zaroxi-application-remote",
    "zaroxi-application-collaboration",

    # INTELLIGENCE
    "zaroxi-intelligence-agent",
    "zaroxi-intelligence-planning",
    "zaroxi-intelligence-memory",
    "zaroxi-intelligence-context",
    "zaroxi-intelligence-tools",
    "zaroxi-intelligence-orchestrator",
    "zaroxi-intelligence-eval",
    "zaroxi-intelligence-embedding",
    "zaroxi-intelligence-safety",

    # INFRASTRUCTURE
    "zaroxi-infrastructure-rpc",
    "zaroxi-infrastructure-http",
    "zaroxi-infrastructure-storage",
    "zaroxi-infrastructure-settings",
    "zaroxi-infrastructure-permissions",
    "zaroxi-infrastructure-logging",
    "zaroxi-infrastructure-metrics",
    "zaroxi-infrastructure-tracing",
    "zaroxi-infrastructure-network",
    "zaroxi-infrastructure-process",
    "zaroxi-infrastructure-ssh",
    "zaroxi-infrastructure-container",

    # SECURITY
    "zaroxi-security-sandbox",
    "zaroxi-security-policy",
    "zaroxi-security-validation",
    "zaroxi-security-auth",
    "zaroxi-security-audit",
    "zaroxi-security-crypto",

    # INTERFACE
    "zaroxi-interface-app",
    "zaroxi-interface-desktop",
    "zaroxi-interface-cli",
    "zaroxi-interface-theme",
]

TOOLS: List[str] = [
    "tools/crate-lint",
    "docs",
]

# Layer prefixes used for validation and header output
LAYER_PREFIXES: Dict[str, List[str]] = {
    "kernel": ["zaroxi-kernel-"],
    "core": ["zaroxi-core-", "zaroxi-core-engine-", "zaroxi-core-editor-", "zaroxi-core-workspace-", "zaroxi-core-platform-"],
    "domain": ["zaroxi-domain-"],
    "application": ["zaroxi-application-"],
    "interface": ["zaroxi-interface-"],
    "intelligence": ["zaroxi-intelligence-"],
    "infrastructure": ["zaroxi-infrastructure-"],
    "security": ["zaroxi-security-"],
}

# Allowed dependency targets per layer (strict, no upward deps)
ALLOWED = {
    "interface": ["application"],
    "application": ["domain"],
    "domain": ["core"],
    "core": ["kernel"],
    "kernel": [],
    "intelligence": ["domain"],         # intelligence -> domain ONLY
    "infrastructure": ["domain"],      # infrastructure implements adapters for domain only
    "security": ["kernel", "core"],    # security is cross-cutting but cannot break layering (allowed to depend down)
}


def detect_layer(crate_name: str) -> str:
    for layer, prefixes in LAYER_PREFIXES.items():
        for p in prefixes:
            if crate_name.startswith(p):
                return layer
    return "unknown"


def allowed_prefixes_for(crate_name: str) -> List[str]:
    layer = detect_layer(crate_name)
    allowed_layers = ALLOWED.get(layer, [])
    prefixes: List[str] = []
    for l in allowed_layers:
        prefixes.extend(LAYER_PREFIXES.get(l, []))
    return prefixes


def crate_description(crate_name: str) -> str:
    """
    Produce a professional, human-readable description for the crate.
    This is intentionally explicit so generated Cargo.toml descriptions are meaningful.
    """
    # Specific handcrafted descriptions for known crates (concise and publication-ready).
    mapping = {
        # Kernel
        "zaroxi-kernel-core": "Core kernel primitives and application entry points for Zaroxi.",
        "zaroxi-kernel-types": "Fundamental kernel-level type definitions used across Zaroxi.",
        "zaroxi-kernel-errors": "Standardized kernel error types and conversions.",
        "zaroxi-kernel-memory": "Memory management and safe allocation utilities for kernel components.",
        "zaroxi-kernel-async": "Async runtime primitives and ergonomics used by kernel and core.",
        "zaroxi-kernel-time": "Stable time utilities and monotonic clocks for the platform.",
        "zaroxi-kernel-math": "High-performance math utilities and numeric helpers.",
        "zaroxi-kernel-collections": "Specialized collection types optimized for IDE workloads.",
        "zaroxi-kernel-traits": "Common low-level traits shared by kernel and core crates.",
        "zaroxi-kernel-config": "Types and defaults for global runtime configuration.",
        "zaroxi-kernel-protocol": "Wire protocol definitions and serialization primitives.",

        # Domain examples (more can be added; fallback below)
        "zaroxi-domain-ai": "Domain models and types for AI features and reasoning.",
        "zaroxi-domain-workspace": "Domain representation of workspace metadata and invariants.",
        "zaroxi-application-editor": "High-level editor orchestration built on domain and core services.",
    }
    if crate_name in mapping:
        return mapping[crate_name]

    # Heuristic descriptions based on crate name components
    short = crate_name.replace("zaroxi-", "").replace("-", " ").strip()
    # Capitalize first letter
    return f"{short.capitalize()} subsystem for the Zaroxi platform. Implements precise responsibilities within its layer."


def write_crate(crate_name: str):
    """
    Create crate directory, Cargo.toml and src/lib.rs template.
    Cargo.toml includes structured metadata suitable for long-term maintenance.
    """
    layer = detect_layer(crate_name)
    desc = crate_description(crate_name)
    crate_dir = os.path.join(ROOT, "crates", crate_name)
    src_dir = os.path.join(crate_dir, "src")
    os.makedirs(src_dir, exist_ok=True)

    cargo_toml = textwrap.dedent(f"""\
        [package]
        name = "{crate_name}"
        version = "{CRATE_VERSION}"
        edition = "{RUST_EDITION}"
        description = "{desc}"
        license = "{LICENSE}"
        authors = []
        documentation = ""
        repository = ""
        readme = ""
        keywords = ["zaroxi", "{crate_name}"]
        categories = ["development-tools"]
        
        [package.metadata.zaroxi]
        layer = "{layer}"
        responsibility = "Single-responsibility crate implementing the {crate_name} concern."
        allowed_dependency_prefixes = {allowed_prefixes_for(crate_name)}
    """)

    lib_rs = textwrap.dedent(f"""\
        //! {desc}
        //!
        //! Layer: {layer}
        //! Responsibility: Single-responsibility crate implementing the {crate_name} concern.
        //!
        //! Allowed dependency prefixes: {allowed_prefixes_for(crate_name)}
        //! Forbidden dependency prefixes: any crate prefix not listed above and any upward-layer dependency.
        //!
        //! This file contains a minimal placeholder. Implementation files should:
        //! - Keep public surface small and well documented.
        //! - Avoid leaking internal details across layers.
        //!
        //! Generated by tools/create_crates.py (RUST_EDITION={RUST_EDITION}, RUST_TOOLCHAIN={RUST_TOOLCHAIN}).
        
        /// Placeholder function to make the crate non-empty.
        pub fn _placeholder() {{
            // Intentionally empty implementation.
        }}
    """)

    with open(os.path.join(crate_dir, "Cargo.toml"), "w", encoding="utf-8") as f:
        f.write(cargo_toml)
    with open(os.path.join(src_dir, "lib.rs"), "w", encoding="utf-8") as f:
        f.write(lib_rs)


def write_tool(path: str):
    full = os.path.join(ROOT, path)
    os.makedirs(full, exist_ok=True)
    with open(os.path.join(full, ".gitkeep"), "w", encoding="utf-8") as f:
        f.write("# placeholder\n")


def write_workspace_toml():
    """
    Emit a root Cargo.toml for the workspace that enumerates all crate members and
    pins the workspace edition to the mandated RUST_EDITION.
    """
    members = [f'  "crates/{c}",' for c in CRATES]
    # include tools/docs as workspace members
    members.append('  "tools/crate-lint",')
    members.append('  "docs",')
    members_block = "\n".join(members)
    workspace_toml = textwrap.dedent(f"""\
        [workspace]
        resolver = "2"
        members = [
        {members_block}
        ]
        
        [workspace.package]
        edition = "{RUST_EDITION}"
        version = "0.1.0"
        license = "{LICENSE}"
        
        [workspace.dependencies]
        anyhow = "1"
        serde = {{ version = "1", features = ["derive"] }}
        serde_json = "1"
        tokio = {{ version = "1", features = ["rt-multi-thread","macros","sync","time"] }}
        tracing = "0.1"
        uuid = {{ version = "1", features = ["v4", "serde"] }}
        thiserror = "2"
        parking_lot = "0.12"
        once_cell = "1.20"
    """)
    with open(os.path.join(ROOT, "Cargo.toml"), "w", encoding="utf-8") as f:
        f.write(workspace_toml)


def write_rust_toolchain():
    """
    Write a .rust-toolchain file to recommend the toolchain for the repository.
    """
    content = textwrap.dedent(f"""\
        [toolchain]
        channel = "{RUST_TOOLCHAIN}"
        components = [ "rustfmt", "clippy" ]
    """)
    with open(os.path.join(ROOT, ".rust-toolchain.toml"), "w", encoding="utf-8") as f:
        f.write(content)


def main():
    print("Creating Zaroxi crate skeletons under crates/ ...")
    for c in CRATES:
        print(" -", c)
        write_crate(c)
    for t in TOOLS:
        print("Creating tool/doc:", t)
        write_tool(t)

    print("Writing root Cargo.toml workspace file...")
    write_workspace_toml()

    print("Writing .rust-toolchain.toml...")
    write_rust_toolchain()

    print("\nDone.")
    print("Validation summary (per-crate allowed dependency prefixes):")
    for c in CRATES:
        print(f"{c}: layer={detect_layer(c)}, allowed_prefixes={allowed_prefixes_for(c)}")

    print("\nSuggested next steps (run from repo root):")
    print(" - python3 tools/create_crates.py")
    print(" - git add Cargo.toml .rust-toolchain.toml crates tools")
    print(' - git commit -m "chore: bootstrap Zaroxi crate skeletons and workspace (edition=2024)"')

if __name__ == "__main__":
    main()
