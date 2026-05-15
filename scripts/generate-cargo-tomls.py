#!/usr/bin/env python3
"""
Generate canonical Cargo.toml files for workspace crates.

- Reads the workspace Cargo.toml in repo root and enumerates members.
- For each member under `crates/` or `tools/`, writes a canonical Cargo.toml with:
  - package.name = basename(member)
  - version = "0.1.0"
  - edition = "2024"
  - license = "MIT"
  - description = taken from an internal mapping (based on the architecture). If no mapping,
    a sensible default is generated from the crate path and prefix.
- Existing Cargo.toml files are backed up to Cargo.toml.bak before overwrite.

Run: python3 scripts/generate-cargo-tomls.py
"""
import os
import re
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKSPACE_MANIFEST = ROOT / "Cargo.toml"

if not WORKSPACE_MANIFEST.exists():
    raise SystemExit("Workspace Cargo.toml not found at repo root.")

s = WORKSPACE_MANIFEST.read_text(encoding="utf-8")

m = re.search(r"members\s*=\s*\[(.*?)\]", s, re.S)
if not m:
    raise SystemExit("No members block found in Cargo.toml")

block = m.group(1)
members = re.findall(r'"([^"]+)"', block)

# Canonical descriptions mapping (from architecture)
desc_map = {
    # kernel
    "zaroxi-kernel-core": "Kernel core primitives for Zaroxi (zero-dependency): strongly-typed IDs and tiny helpers.",
    "zaroxi-kernel-types": "Kernel types: Id, Position, Span, Range; small serializable primitives.",
    "zaroxi-kernel-errors": "Unified kernel error types and conversion utilities.",
    "zaroxi-kernel-memory": "Arena allocators and memory pools (kernel-level).",
    "zaroxi-kernel-async": "Small async/task primitives usable without pulling full runtimes.",
    "zaroxi-kernel-time": "Monotonic time wrappers and timers for kernels/core.",
    "zaroxi-kernel-math": "Geometry and layout math utilities for rendering and layout.",
    "zaroxi-kernel-collections": "Optimized collections and lock-free containers.",
    "zaroxi-kernel-traits": "Minimal shared trait definitions used across layers.",
    "zaroxi-kernel-config": "Canonical config schema types used by kernel and higher layers (no IO).",

    # core
    "zaroxi-core-input": "Normalized input events and raw event queue (keyboard/mouse/pen).",
    "zaroxi-core-event": "Event bus and typed event propagation primitives.",
    "zaroxi-core-commands": "Typed command registry, execution model and undo hooks.",
    "zaroxi-core-runtime": "Core runtime: task scheduling and render-loop orchestration.",
    "zaroxi-core-scheduler": "High-performance scheduler for short-lived UI/IO tasks.",
    "zaroxi-core-task": "Task primitives used across core and application layers.",
    "zaroxi-core-threading": "Cross-platform threading primitives and thread pools.",
    "zaroxi-core-text": "Facade API for text buffers, edits and snapshots (zero-copy).",
    "zaroxi-core-text-buffer": "High-level buffer model with snapshots and change history.",
    "zaroxi-core-text-rope": "Rope data structure optimized for huge files and zero-copy slices.",
    "zaroxi-core-text-edit": "Efficient edit application and range translation utilities.",
    "zaroxi-core-text-diff": "Incremental diff algorithms for very large files.",
    "zaroxi-core-render": "GPU rendering API abstraction and renderer facade.",
    "zaroxi-core-render-backend": "Backend adapters (wgpu/vulkan/metal) behind feature flags.",
    "zaroxi-core-render-graph": "Render graph system for resource and pass scheduling.",
    "zaroxi-core-render-pipeline": "Shader pipeline and pipeline state management.",
    "zaroxi-core-render-resource": "GPU resource management and lifetime tracking.",
    "zaroxi-core-render-text": "Glyph shaping, atlas management and GPU text uploading.",
    "zaroxi-core-render-ui": "Immediate/retained UI primitives and batching system.",
    "zaroxi-core-render-scene": "Scene graph for compositing overlays and complex visuals.",
    "zaroxi-core-render-compositor": "Final composition & post-processing pipelines.",
    "zaroxi-core-layout": "Layout primitives (flex-like) and measurement utilities.",
    "zaroxi-core-ui": "Low-level widgets and input integration (core UI primitives).",

    # domain
    "zaroxi-domain-editor": "Editor domain: documents, cursors, selections and history.",
    "zaroxi-domain-workspace": "Workspace model and project metadata (pure logic, no IO).",
    "zaroxi-domain-project": "Project model, dependency graph, and build targets.",
    "zaroxi-domain-buffer": "Domain buffer semantics tying domain logic to core text model.",
    "zaroxi-domain-selection": "Selection model and multi-cursor semantics.",
    "zaroxi-domain-cursor": "Cursor movement semantics and visibility logic.",
    "zaroxi-domain-history": "Change history and checkpointing logic.",
    "zaroxi-domain-ai": "AI domain models: context collection and prompt builders.",
    "zaroxi-domain-settings": "Canonical settings model (no IO).",

    # application
    "zaroxi-application-editor": "Application orchestration for editor features (commands → domain → core).",
    "zaroxi-application-workspace": "Workspace orchestration: open/close, indexing triggers and watchers.",
    "zaroxi-application-project": "Build and task orchestration for projects.",
    "zaroxi-application-buffer": "Buffer lifecycle and syncing between domain and core text buffers.",
    "zaroxi-application-command": "Command registry glue and permission checks.",
    "zaroxi-application-search": "Cross-file search orchestration and indexing.",
    "zaroxi-application-ai": "High-level AI orchestration: copilots and assistant features.",
    "zaroxi-application-navigation": "Navigation features: go-to-definition and symbol indices.",
    "zaroxi-application-refactor": "Refactoring orchestration and safe apply mechanisms.",

    # intelligence
    "zaroxi-intelligence-agent": "Agent runtime for planners and executors (AI agents core).",
    "zaroxi-intelligence-planning": "Plan generation and step decomposition algorithms.",
    "zaroxi-intelligence-memory": "In-memory vector DB and memory cache patterns.",
    "zaroxi-intelligence-context": "Context packing and prompt building for agents.",
    "zaroxi-intelligence-tools": "Trait-only tool interfaces for agents (filesystem, git).",
    "zaroxi-intelligence-orchestrator": "Multi-agent coordination and scheduling.",
    "zaroxi-intelligence-eval": "Evaluation harnesses for agent outputs.",
    "zaroxi-intelligence-embedding": "Embedding utilities and interfaces (local).",

    # platform
    "zaroxi-platform-lsp": "Language Server Protocol adapter and session management.",
    "zaroxi-platform-syntax": "Language grammars and parser runtime (Tree-sitter integration).",
    "zaroxi-platform-debugger": "Debugger protocol adapters and session management.",
    "zaroxi-platform-terminal": "Terminal emulator integration.",
    "zaroxi-platform-git": "Git model and trait-only adapters.",
    "zaroxi-platform-test": "Test integration and harnesses.",
    "zaroxi-platform-profiler": "Profiler hooks and integrations.",
    "zaroxi-platform-formatter": "Formatter adapter integrations.",
    "zaroxi-platform-linter": "Linter integrations and adapters.",
    "zaroxi-platform-plugin": "Plugin host API and sandbox contracts.",

    # workspace
    "zaroxi-workspace-files": "Workspace file model and file-loading strategies (no IO in model).",
    "zaroxi-workspace-index": "Indexing model and metadata for fast search.",
    "zaroxi-workspace-history": "Workspace-level operation timeline and undo.",
    "zaroxi-workspace-patch": "Patch model and preview application (pure logic).",
    "zaroxi-workspace-watcher": "Trait-only watcher interface; platform impl in infra.",
    "zaroxi-workspace-cache": "Caching policy interfaces and in-memory cache impl.",

    # collaboration
    "zaroxi-collaboration-sync": "Session sync orchestration for real-time collaboration.",
    "zaroxi-collaboration-crdt": "CRDT implementations for text and presence.",
    "zaroxi-collaboration-presence": "Presence model and ephemeral state.",
    "zaroxi-collaboration-session": "Session lifecycle and permissions integration.",

    # infrastructure
    "zaroxi-infrastructure-rpc": "RPC transport adapters (JSON-RPC / protobuf) for server/client.",
    "zaroxi-infrastructure-http": "HTTP & websocket transport adapters.",
    "zaroxi-infrastructure-storage": "FS and remote storage adapters (S3/GCS) behind traits.",
    "zaroxi-infrastructure-settings": "Settings persistence and loader adapters.",
    "zaroxi-infrastructure-permissions": "Permission evaluation engine and RBAC mappings.",
    "zaroxi-infrastructure-logging": "Logging sink adapters (OpenTelemetry, file).",
    "zaroxi-infrastructure-metrics": "Metrics exporters and collectors.",
    "zaroxi-infrastructure-tracing": "Tracing adapters (OpenTelemetry).",

    # security
    "zaroxi-security-sandbox": "Sandbox abstraction for running plugins and agents.",
    "zaroxi-security-policy": "Permission language and policy evaluation engine.",
    "zaroxi-security-validation": "Artifact validation and integrity checks for plugins.",
    "zaroxi-security-auth": "Authentication flows and token validation (pluggable).",

    # interface
    "zaroxi-interface-app": "Desktop/web app shell and orchestration (entrypoint).",
    "zaroxi-interface-editor": "Concrete editor UI combining core-ui and domain models.",
    "zaroxi-interface-theme": "Theme primitives, semantic colors and design tokens.",
    "zaroxi-interface-cli": "CLI entrypoints for headless operations.",
    "zaroxi-interface-gui": "GUI toolkit integration and native windowing entrypoints.",

    # tools
    "tools-generate-stubs": "Tool: generate missing crate stubs from workspace members.",
    "tools-crate-lint": "Tool: crate dependency and layer enforcement utility (placeholder).",
}

# helper to produce default description when missing
def default_description(pkg_name):
    if pkg_name.startswith("zaroxi-kernel"):
        return "Kernel crate: low-level primitives (zero-dependency)."
    if pkg_name.startswith("zaroxi-core"):
        return "Core crate: low-level engine component."
    if pkg_name.startswith("zaroxi-domain"):
        return "Domain crate: pure business logic (no IO)."
    if pkg_name.startswith("zaroxi-application"):
        return "Application crate: orchestration and use-case logic."
    if pkg_name.startswith("zaroxi-intelligence"):
        return "Intelligence crate: AI-first models and tooling."
    if pkg_name.startswith("zaroxi-platform"):
        return "Platform crate: language/tooling adapter."
    if pkg_name.startswith("zaroxi-infrastructure") or pkg_name.startswith("zaroxi-infra"):
        return "Infrastructure crate: external adapter implementation (IO)."
    if pkg_name.startswith("zaroxi-security"):
        return "Security crate: sandboxing, policy and auth."
    if pkg_name.startswith("zaroxi-interface") or pkg_name.startswith("zaroxi-app"):
        return "Interface crate: UI or entrypoint glue."
    if pkg_name.startswith("tools"):
        return "Tooling crate for workspace maintenance."
    return f"Zaroxi crate `{pkg_name}` (auto-generated description)."

def write_cargo_toml(crate_path: Path, pkg_name: str, description: str):
    cargo_toml_path = crate_path / "Cargo.toml"
    # backup
    if cargo_toml_path.exists():
        bak = cargo_toml_path.with_suffix(".toml.bak")
        shutil.copy2(cargo_toml_path, bak)
    content = f"""[package]
name = "{pkg_name}"
version = "0.1.0"
edition = "2024"
license = "MIT"
description = "{description}"
rust-version = "1.70"

[dependencies]
# Add crate-specific dependencies when replacing stubbed manifests.
"""
    cargo_toml_path.write_text(content, encoding="utf-8")
    print(f"wrote: {cargo_toml_path}")

for mem in members:
    mem = mem.strip()
    if not mem:
        continue
    # skip non-crate paths
    if mem.startswith("docs") or mem.startswith(".github") or mem.startswith("tools/generate-stubs") or mem in ("tools", "docs"):
        continue
    crate_path = ROOT / mem
    pkg_name = Path(mem).name
    # derive description
    desc = desc_map.get(pkg_name, default_description(pkg_name))
    # Ensure crate dir exists
    crate_path.mkdir(parents=True, exist_ok=True)
    # write Cargo.toml
    write_cargo_toml(crate_path, pkg_name, desc)

print("Done: canonical Cargo.toml files generated/updated for workspace members.")
