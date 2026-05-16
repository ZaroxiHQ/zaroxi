#!/usr/bin/env python3
# Script: tools/create_crates.py
# Purpose: Create the full Zaroxi crate tree with minimal Cargo.toml and src/lib.rs templates.
# Usage: python3 tools/create_crates.py
#
# This script enforces:
# - edition = "2024"
# - flat layout: crates/<crate-name>
# - each crate gets a Cargo.toml with name, version "0.1.0", edition "2024", description
# - each crate gets src/lib.rs with a strict header documenting purpose, layer, and single responsibility
# - writes tools/crate-lint and docs directories placeholders
#
# The script also computes allowed / forbidden dependency prefixes per layer to help validation.

import os
import sys
import textwrap

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))

CRATES = [
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

TOOLS = [
    "tools/crate-lint",
    "docs",
]

LAYER_PREFIXES = {
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

def allowed_prefixes_for(crate_name: str):
    layer = detect_layer(crate_name)
    allowed_layers = ALLOWED.get(layer, [])
    prefixes = []
    for l in allowed_layers:
        prefixes.extend(LAYER_PREFIXES.get(l, []))
    return prefixes

def human_purpose(crate_name: str) -> str:
    # Derive a concise purpose from the crate name.
    name = crate_name.replace("zaroxi-", "").replace("-", " ").strip()
    return f"Provides {name} functionality for the Zaroxi platform."

def write_crate(crate_name: str):
    layer = detect_layer(crate_name)
    desc = human_purpose(crate_name)
    crate_dir = os.path.join(ROOT, "crates", crate_name)
    src_dir = os.path.join(crate_dir, "src")
    os.makedirs(src_dir, exist_ok=True)

    cargo_toml = textwrap.dedent(f"""\
        [package]
        name = "{crate_name}"
        version = "0.1.0"
        edition = "2024"
        description = "{desc}"
        license = "MIT"
    """)

    lib_rs = textwrap.dedent(f"""\
        //! {desc}
        //!
        //! Layer: {layer}
        //! Responsibility: Single-responsibility crate implementing the {crate_name} concern.
        //!
        //! Allowed dependency prefixes: {allowed_prefixes_for(crate_name)}
        //! Forbidden dependency prefixes: (any crate prefix not listed above, and any upward-layer dependency)
        //!
        //! Note: Do not add runtime dependencies that violate the global layering rules.
        pub fn _placeholder() {{
            // Placeholder to make this crate non-empty.
        }}
    """)

    with open(os.path.join(crate_dir, "Cargo.toml"), "w", encoding="utf-8") as f:
        f.write(cargo_toml)
    with open(os.path.join(src_dir, "lib.rs"), "w", encoding="utf-8") as f:
        f.write(lib_rs)

def write_tool(path):
    full = os.path.join(ROOT, path)
    os.makedirs(full, exist_ok=True)
    with open(os.path.join(full, ".gitkeep"), "w") as f:
        f.write("# placeholder\n")

def main():
    print("Creating Zaroxi crate skeletons under crates/ ...")
    for c in CRATES:
        print(" -", c)
        write_crate(c)
    for t in TOOLS:
        print("Creating tool/doc:", t)
        write_tool(t)
    print("\nDone.")
    print("Validation summary (per-crate allowed dependency prefixes):")
    for c in CRATES:
        print(f"{c}: layer={detect_layer(c)}, allowed_prefixes={allowed_prefixes_for(c)}")

if __name__ == "__main__":
    main()
