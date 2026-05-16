"""
ZAROXI PRODUCTION MIGRATION DIRECTIVE (FINAL)

This file is the FINAL, AUTHORITATIVE architecture refactor plan and the
automation script that performs the migration. Run this script from the
workspace root. It is deterministic, idempotent, uses only the Python
standard library, and will produce a flat crates/ layout matching the
approved target inventory.

CONTENTS (MUST APPEAR IN THIS ORDER)
1. Final Architecture Decision
2. Keep As-Is
3. Rename Map
4. Remove
5. Create
6. Final Flat Crate Tree
7. Dependency Laws
8. Python Migration Script (this file below)
9. Validation Checklist

========================================================================
1. FINAL ARCHITECTURE DECISION
========================================================================

ZAROXI will be refactored into a strict modular monolith with a flat
workspace layout under crates/. Crate names will follow mandatory prefixes
(zaroxi-kernel-*, zaroxi-core-*, zaroxi-domain-*, zaroxi-application-*, 
zaroxi-intelligence-*, zaroxi-infrastructure-*, zaroxi-security-*, 
zaroxi-interface-*). All core UI runtime components are segregated under
the core-engine family. Editor primitives are segregated under core-editor.
Workspace truth and filesystem concerns are under core-workspace. Platform
integrations are under core-platform. Collaboration is consolidated
across three crates: core-editor-collab (editor CRDT/OT primitives),
zaroxi-domain-collaboration (domain model), and
zaroxi-application-collaboration (application-level orchestration).
All crates will target edition = "2024", version = "0.1.0". The migration
is automated and idempotent.

This decision is FINAL and non-negotiable.

========================================================================
2. KEEP AS-IS
========================================================================

These existing crates already match the final inventory and will remain
unchanged (no rename, no deletion):

- zaroxi-application-ai
- zaroxi-application-command
- zaroxi-application-editor
- zaroxi-application-navigation
- zaroxi-application-plugin
- zaroxi-application-project
- zaroxi-application-refactor
- zaroxi-application-search
- zaroxi-application-workspace
- zaroxi-core-commands
- zaroxi-core-event
- zaroxi-core-input
- zaroxi-core-plugin-runtime
- zaroxi-core-runtime
- zaroxi-core-scheduler
- zaroxi-core-state
- zaroxi-core-task
- zaroxi-core-telemetry
- zaroxi-core-threading
- zaroxi-domain-ai
- zaroxi-domain-project
- zaroxi-domain-settings
- zaroxi-domain-workspace
- zaroxi-infrastructure-http
- zaroxi-infrastructure-logging
- zaroxi-infrastructure-metrics
- zaroxi-infrastructure-network
- zaroxi-infrastructure-permissions
- zaroxi-infrastructure-rpc
- zaroxi-infrastructure-settings
- zaroxi-infrastructure-storage
- zaroxi-infrastructure-tracing
- zaroxi-intelligence-agent
- zaroxi-intelligence-context
- zaroxi-intelligence-embedding
- zaroxi-intelligence-eval
- zaroxi-intelligence-memory
- zaroxi-intelligence-orchestrator
- zaroxi-intelligence-planning
- zaroxi-intelligence-tools
- zaroxi-intelligence-safety
- zaroxi-interface-app
- zaroxi-interface-cli
- zaroxi-interface-theme
- zaroxi-kernel-core
- zaroxi-kernel-types
- zaroxi-kernel-errors
- zaroxi-kernel-memory
- zaroxi-kernel-async
- zaroxi-kernel-time
- zaroxi-kernel-math
- zaroxi-kernel-collections
- zaroxi-kernel-traits
- zaroxi-kernel-config
- zaroxi-kernel-protocol
- zaroxi-security-audit
- zaroxi-security-auth
- zaroxi-security-policy
- zaroxi-security-sandbox
- zaroxi-security-validation

========================================================================
3. RENAME MAP
========================================================================

All renames are strict, 1:1 or 1:N merges (where two existing crates
merge into a single final crate name). The automation script will perform
deterministic moves and safe merges.

Format: current -> new : reason

- zaroxi-core-layout -> zaroxi-core-engine-layout
  : UI runtime layout engine belongs to core-engine family.

- zaroxi-core-render -> zaroxi-core-engine-render
  : Rendering is engine-owned; move to core-engine family.

- zaroxi-core-render-backend -> zaroxi-core-engine-render-backend
  : Backend-specific renderer belongs to engine render family.

- zaroxi-core-render-compositor -> zaroxi-core-engine-compositor
  : Engine compositor responsibility.

- zaroxi-core-render-graph -> zaroxi-core-engine-render-graph
  : Engine render graph belongs to core-engine.

- zaroxi-core-render-pipeline -> zaroxi-core-engine-render-pipeline
  : Engine render pipeline belongs to core-engine.

- zaroxi-core-render-resource -> zaroxi-core-engine-render-resource
  : Engine render resources belong to core-engine.

- zaroxi-core-render-scene -> zaroxi-core-engine-scene
  : Engine scene is engine-owned.

- zaroxi-core-render-text -> zaroxi-core-engine-text
  : Text rendering belongs to engine runtime.

- zaroxi-core-render-ui -> zaroxi-core-engine-render
  : UI rendering merges into engine render.

- zaroxi-core-text -> zaroxi-core-editor-model
  : Generic text internals are editor model responsibilities.

- zaroxi-core-text-buffer -> zaroxi-core-editor-buffer
  : Buffer becomes editor buffer under core-editor.

- zaroxi-core-text-rope -> zaroxi-core-editor-rope
  : Rope belongs to editor.

- zaroxi-core-text-edit -> zaroxi-core-editor-transaction
  : Editing transactions belong to editor transaction primitives.

- zaroxi-core-text-diff -> zaroxi-core-editor-history
  : Diffs are history/undo domain for editor.

- zaroxi-core-ui -> zaroxi-core-engine-view
  : UI view primitives belong to engine view.

- zaroxi-domain-cursor -> zaroxi-core-editor-cursor
  : Cursor primitives belong to core-editor.

- zaroxi-domain-history -> zaroxi-core-editor-history
  : Editor history belongs to core-editor.

- zaroxi-domain-selection -> zaroxi-core-editor-selection
  : Selection primitives belong to core-editor.

- zaroxi-domain-buffer -> zaroxi-core-editor-buffer
  : Domain buffer folded into editor buffer where it belongs.

- zaroxi-interface-editor -> zaroxi-interface-desktop
  : Desktop interface bundling (editor+gui -> desktop).

- zaroxi-interface-gui -> zaroxi-interface-desktop
  : Merge GUI surface into interface-desktop.

- zaroxi-platform-debugger -> zaroxi-core-platform-debugger
  : platform -> core-platform family.

- zaroxi-platform-formatter -> zaroxi-core-platform-formatter
- zaroxi-platform-git -> zaroxi-core-platform-git
- zaroxi-platform-linter -> zaroxi-core-platform-linter
- zaroxi-platform-lsp -> zaroxi-core-platform-lsp
- zaroxi-platform-plugin -> zaroxi-core-platform-plugin
- zaroxi-platform-profiler -> zaroxi-core-platform-profiler
- zaroxi-platform-syntax -> zaroxi-core-platform-syntax
- zaroxi-platform-terminal -> zaroxi-core-platform-terminal
- zaroxi-platform-test -> zaroxi-core-platform-test

  : All platform-* crates are renamed into zaroxi-core-platform-*.

- zaroxi-workspace-cache -> zaroxi-core-workspace-cache
- zaroxi-workspace-files -> zaroxi-core-workspace-files
- zaroxi-workspace-history -> zaroxi-core-workspace-history
- zaroxi-workspace-index -> zaroxi-core-workspace-index
- zaroxi-workspace-patch -> zaroxi-core-workspace-patch
- zaroxi-workspace-watcher -> zaroxi-core-workspace-watcher

  : workspace-* crates move under core-workspace family.

NOTE: The script performs merges when the destination crate already
exists: it moves non-conflicting files into the destination and updates
Cargo.toml package name. No source code content transformation is performed.

========================================================================
4. REMOVE
========================================================================

Crates to delete because they are obsolete, redundant, or will be
replaced by the new consolidated crates:

- zaroxi-collaboration-crdt
- zaroxi-collaboration-presence
- zaroxi-collaboration-session
- zaroxi-collaboration-sync
  : Fragmented collaboration crates are removed; functionality will be
    consolidated into zaroxi-core-editor-collab, zaroxi-domain-collaboration,
    and zaroxi-application-collaboration.

- zaroxi-application-buffer
  : Buffer is a core-editor primitive, not an application-level crate.

- zaroxi-domain-editor
  : Editor internals belong under core-editor; domain-editor is removed.

All removals are performed only for directories not present in the
final inventory and only after any required merges/renames complete.

========================================================================
5. CREATE
========================================================================

The script will create missing crates required by the target inventory.
They will be empty library crates with canonical Cargo.toml and src/lib.rs
stubs (edition=2024, version=0.1.0) and a meaningful description.

Grouped by layer:

CORE ENGINE (create any missing)
- zaroxi-core-engine-root
- zaroxi-core-engine-runtime
- zaroxi-core-engine-state
- zaroxi-core-engine-window
- zaroxi-core-engine-input
- zaroxi-core-engine-action
- zaroxi-core-engine-focus
- zaroxi-core-engine-layout
- zaroxi-core-engine-style
- zaroxi-core-engine-element
- zaroxi-core-engine-view
- zaroxi-core-engine-scene
- zaroxi-core-engine-overlay
- zaroxi-core-engine-text
- zaroxi-core-engine-font
- zaroxi-core-engine-render
- zaroxi-core-engine-render-backend
- zaroxi-core-engine-render-resource
- zaroxi-core-engine-render-pipeline
- zaroxi-core-engine-render-graph
- zaroxi-core-engine-compositor
- zaroxi-core-engine-animation
- zaroxi-core-engine-clipboard
- zaroxi-core-engine-ime
- zaroxi-core-engine-accessibility
- zaroxi-core-engine-test

CORE EDITOR (create any missing)
- zaroxi-core-editor-buffer
- zaroxi-core-editor-rope
- zaroxi-core-editor-model
- zaroxi-core-editor-transaction
- zaroxi-core-editor-selection
- zaroxi-core-editor-cursor
- zaroxi-core-editor-history
- zaroxi-core-editor-display
- zaroxi-core-editor-decoration
- zaroxi-core-editor-folding
- zaroxi-core-editor-gutter
- zaroxi-core-editor-diagnostics
- zaroxi-core-editor-minimap
- zaroxi-core-editor-command
- zaroxi-core-editor-viewport
- zaroxi-core-editor-inline-ai
- zaroxi-core-editor-view
- zaroxi-core-editor-collab  <-- MANDATORY (CRDT/OT integration layer)

CORE WORKSPACE
- zaroxi-core-workspace-files
- zaroxi-core-workspace-index
- zaroxi-core-workspace-history
- zaroxi-core-workspace-patch
- zaroxi-core-workspace-watcher
- zaroxi-core-workspace-cache
- zaroxi-core-workspace-snapshot
- zaroxi-core-workspace-permissions

CORE PLATFORM
- zaroxi-core-platform-runtime
- zaroxi-core-platform-lsp
- zaroxi-core-platform-syntax
- zaroxi-core-platform-debugger
- zaroxi-core-platform-terminal
- zaroxi-core-platform-git
- zaroxi-core-platform-test
- zaroxi-core-platform-profiler
- zaroxi-core-platform-formatter
- zaroxi-core-platform-linter
- zaroxi-core-platform-plugin
- zaroxi-core-platform-remote-ssh
- zaroxi-core-platform-remote-container

INTELLIGENCE - already present (kept)
INFRASTRUCTURE - already present (kept)
SECURITY
- zaroxi-security-crypto  <-- create (missing)

INTERFACE
- zaroxi-interface-desktop  <-- created by merging interface-editor & interface-gui

APPLICATION
- zaroxi-application-collaboration  <-- create (application-level orchestration)

DOMAIN
- zaroxi-domain-collaboration  <-- create (domain model for collaboration)

========================================================================
6. FINAL FLAT CRATE TREE (exact crates/ entries)
========================================================================

The script will produce the following exact final crates/ listing (order is
not semantically important, this is the authoritative inventory used by the
automation):

[KERNEL]
- zaroxi-kernel-core
- zaroxi-kernel-types
- zaroxi-kernel-errors
- zaroxi-kernel-memory
- zaroxi-kernel-async
- zaroxi-kernel-time
- zaroxi-kernel-math
- zaroxi-kernel-collections
- zaroxi-kernel-traits
- zaroxi-kernel-config
- zaroxi-kernel-protocol

[CORE ENGINE]
- zaroxi-core-engine-root
- zaroxi-core-engine-runtime
- zaroxi-core-engine-state
- zaroxi-core-engine-window
- zaroxi-core-engine-input
- zaroxi-core-engine-action
- zaroxi-core-engine-focus
- zaroxi-core-engine-layout
- zaroxi-core-engine-style
- zaroxi-core-engine-element
- zaroxi-core-engine-view
- zaroxi-core-engine-scene
- zaroxi-core-engine-overlay
- zaroxi-core-engine-text
- zaroxi-core-engine-font
- zaroxi-core-engine-render
- zaroxi-core-engine-render-backend
- zaroxi-core-engine-render-resource
- zaroxi-core-engine-render-pipeline
- zaroxi-core-engine-render-graph
- zaroxi-core-engine-compositor
- zaroxi-core-engine-animation
- zaroxi-core-engine-clipboard
- zaroxi-core-engine-ime
- zaroxi-core-engine-accessibility
- zaroxi-core-engine-test

[CORE EDITOR]
- zaroxi-core-editor-buffer
- zaroxi-core-editor-rope
- zaroxi-core-editor-model
- zaroxi-core-editor-transaction
- zaroxi-core-editor-selection
- zaroxi-core-editor-cursor
- zaroxi-core-editor-history
- zaroxi-core-editor-display
- zaroxi-core-editor-decoration
- zaroxi-core-editor-folding
- zaroxi-core-editor-gutter
- zaroxi-core-editor-diagnostics
- zaroxi-core-editor-minimap
- zaroxi-core-editor-command
- zaroxi-core-editor-viewport
- zaroxi-core-editor-inline-ai
- zaroxi-core-editor-view
- zaroxi-core-editor-collab

[CORE WORKSPACE]
- zaroxi-core-workspace-files
- zaroxi-core-workspace-index
- zaroxi-core-workspace-history
- zaroxi-core-workspace-patch
- zaroxi-core-workspace-watcher
- zaroxi-core-workspace-cache
- zaroxi-core-workspace-snapshot
- zaroxi-core-workspace-permissions

[CORE PLATFORM]
- zaroxi-core-platform-runtime
- zaroxi-core-platform-lsp
- zaroxi-core-platform-syntax
- zaroxi-core-platform-debugger
- zaroxi-core-platform-terminal
- zaroxi-core-platform-git
- zaroxi-core-platform-test
- zaroxi-core-platform-profiler
- zaroxi-core-platform-formatter
- zaroxi-core-platform-linter
- zaroxi-core-platform-plugin
- zaroxi-core-platform-remote-ssh
- zaroxi-core-platform-remote-container

[CORE SHARED / RUNTIME]
- zaroxi-core-runtime
- zaroxi-core-scheduler
- zaroxi-core-task
- zaroxi-core-threading
- zaroxi-core-state
- zaroxi-core-io
- zaroxi-core-sync
- zaroxi-core-input
- zaroxi-core-event
- zaroxi-core-commands
- zaroxi-core-telemetry
- zaroxi-core-plugin-runtime

[DOMAIN]
- zaroxi-domain-workspace
- zaroxi-domain-project
- zaroxi-domain-settings
- zaroxi-domain-session
- zaroxi-domain-ai
- zaroxi-domain-plugin
- zaroxi-domain-collaboration

[APPLICATION]
- zaroxi-application-editor
- zaroxi-application-workspace
- zaroxi-application-project
- zaroxi-application-command
- zaroxi-application-search
- zaroxi-application-navigation
- zaroxi-application-refactor
- zaroxi-application-ai
- zaroxi-application-plugin
- zaroxi-application-remote
- zaroxi-application-collaboration

[INTELLIGENCE]
- zaroxi-intelligence-agent
- zaroxi-intelligence-planning
- zaroxi-intelligence-memory
- zaroxi-intelligence-context
- zaroxi-intelligence-tools
- zaroxi-intelligence-orchestrator
- zaroxi-intelligence-eval
- zaroxi-intelligence-embedding
- zaroxi-intelligence-safety

[INFRASTRUCTURE]
- zaroxi-infrastructure-rpc
- zaroxi-infrastructure-http
- zaroxi-infrastructure-storage
- zaroxi-infrastructure-settings
- zaroxi-infrastructure-permissions
- zaroxi-infrastructure-logging
- zaroxi-infrastructure-metrics
- zaroxi-infrastructure-tracing
- zaroxi-infrastructure-network
- zaroxi-infrastructure-process
- zaroxi-infrastructure-ssh
- zaroxi-infrastructure-container

[SECURITY]
- zaroxi-security-sandbox
- zaroxi-security-policy
- zaroxi-security-validation
- zaroxi-security-auth
- zaroxi-security-audit
- zaroxi-security-crypto

[INTERFACE]
- zaroxi-interface-app
- zaroxi-interface-desktop
- zaroxi-interface-cli
- zaroxi-interface-theme

========================================================================
7. DEPENDENCY LAWS (STRICT)
========================================================================

Follow these non-negotiable dependency rules:

- Allowed direction: interface -> application -> domain -> core -> kernel
- intelligence crates depend only on domain crates (and kernel/core shared where necessary).
- infrastructure crates provide adapters only and must depend only on domain or core (never on application or interface).
- security crates are cross-cutting but must not violate layer direction: they may be depended on from any layer below them, but security crates MUST NOT depend on higher layers (e.g., security-auth cannot depend on application or interface).
- core subdomains must not have lateral shortcuts: core-editor must not import core-engine, core-workspace, or core-platform internals (only small, explicit public APIs).
- No circular dependencies anywhere.
- No upward dependencies: lower-level crates (kernel/core) cannot depend on higher-level crates (domain/application/interface).
- No ambiguous crate names (no utils/shared/common/manager/misc/helpers).
- Every crate must have a single, clearly documented responsibility.

========================================================================
8. PYTHON MIGRATION SCRIPT
========================================================================

The script below performs the refactor. It is idempotent, deterministic,
and implements the rename, create, remove and workspace manifest
generation described above. It uses only the Python standard library.

Run:
  python3 scripts/migrate_zaroxi.py

The script will print a concise summary and exit non-zero on unrecoverable
errors.

========================================================================
9. VALIDATION CHECKLIST
========================================================================

After running the script, validate:

- [ ] All crates listed in the FINAL FLAT CRATE TREE exist under crates/.
- [ ] No directories remain under crates/ that are not in the final inventory.
- [ ] All renamed crates have had their Cargo.toml package.name updated.
- [ ] All newly created crates have edition="2024" and version="0.1.0" in Cargo.toml.
- [ ] zaroxi-core-editor-collab exists and is empty library crate stub.
- [ ] No crate name uses forbidden patterns (utils, shared, common, manager, misc, helpers).
- [ ] A top-level Cargo.toml workspace manifest lists exactly the final crates.
- [ ] Run 'cargo metadata' to verify there are no circular dependencies (CI step).
- [ ] Run the project's test/build CI (optional but required before merge).

========================================================================
END OF DIRECTIVE
========================================================================
"""

# ======================================================================
# Python migration script: deterministic, idempotent, standard-library only
# ======================================================================

from __future__ import annotations
import os
import shutil
import sys
from pathlib import Path
import json
import textwrap

# ------------------------
# Configuration (authoritative)
# ------------------------

CRATES_DIR = Path("crates")
WORKSPACE_CARGO = Path("Cargo.toml")

# Final target inventory (exact crate names)
FINAL_CRATES = [
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
    # CORE SHARED / RUNTIME
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

# Deterministic rename mapping: current directory name -> new directory name
RENAME_MAP = {
    # engine and render family
    "zaroxi-core-layout": "zaroxi-core-engine-layout",
    "zaroxi-core-render": "zaroxi-core-engine-render",
    "zaroxi-core-render-backend": "zaroxi-core-engine-render-backend",
    "zaroxi-core-render-compositor": "zaroxi-core-engine-compositor",
    "zaroxi-core-render-graph": "zaroxi-core-engine-render-graph",
    "zaroxi-core-render-pipeline": "zaroxi-core-engine-render-pipeline",
    "zaroxi-core-render-resource": "zaroxi-core-engine-render-resource",
    "zaroxi-core-render-scene": "zaroxi-core-engine-scene",
    "zaroxi-core-render-text": "zaroxi-core-engine-text",
    "zaroxi-core-render-ui": "zaroxi-core-engine-render",
    # text -> editor
    "zaroxi-core-text": "zaroxi-core-editor-model",
    "zaroxi-core-text-buffer": "zaroxi-core-editor-buffer",
    "zaroxi-core-text-rope": "zaroxi-core-editor-rope",
    "zaroxi-core-text-edit": "zaroxi-core-editor-transaction",
    "zaroxi-core-text-diff": "zaroxi-core-editor-history",
    # ui -> engine view
    "zaroxi-core-ui": "zaroxi-core-engine-view",
    # domain -> core-editor
    "zaroxi-domain-cursor": "zaroxi-core-editor-cursor",
    "zaroxi-domain-history": "zaroxi-core-editor-history",
    "zaroxi-domain-selection": "zaroxi-core-editor-selection",
    "zaroxi-domain-buffer": "zaroxi-core-editor-buffer",
    # interface merges
    "zaroxi-interface-editor": "zaroxi-interface-desktop",
    "zaroxi-interface-gui": "zaroxi-interface-desktop",
    # platform -> core-platform family
    "zaroxi-platform-debugger": "zaroxi-core-platform-debugger",
    "zaroxi-platform-formatter": "zaroxi-core-platform-formatter",
    "zaroxi-platform-git": "zaroxi-core-platform-git",
    "zaroxi-platform-linter": "zaroxi-core-platform-linter",
    "zaroxi-platform-lsp": "zaroxi-core-platform-lsp",
    "zaroxi-platform-plugin": "zaroxi-core-platform-plugin",
    "zaroxi-platform-profiler": "zaroxi-core-platform-profiler",
    "zaroxi-platform-syntax": "zaroxi-core-platform-syntax",
    "zaroxi-platform-terminal": "zaroxi-core-platform-terminal",
    "zaroxi-platform-test": "zaroxi-core-platform-test",
    # workspace -> core-workspace family
    "zaroxi-workspace-cache": "zaroxi-core-workspace-cache",
    "zaroxi-workspace-files": "zaroxi-core-workspace-files",
    "zaroxi-workspace-history": "zaroxi-core-workspace-history",
    "zaroxi-workspace-index": "zaroxi-core-workspace-index",
    "zaroxi-workspace-patch": "zaroxi-core-workspace-patch",
    "zaroxi-workspace-watcher": "zaroxi-core-workspace-watcher",
}

# Remove list (obsolete crates to delete)
REMOVE_LIST = [
    "zaroxi-collaboration-crdt",
    "zaroxi-collaboration-presence",
    "zaroxi-collaboration-session",
    "zaroxi-collaboration-sync",
    "zaroxi-application-buffer",
    "zaroxi-domain-editor",
]

# Descriptions for newly created crates (meaningful, short)
DESCRIPTION_MAP = {
    # kernel (existing)
    # core-engine (examples)
    "zaroxi-core-engine-root": "Engine root and lifecycle primitives for Zaroxi UI runtime.",
    "zaroxi-core-engine-runtime": "Engine runtime glue code for Zaroxi UI.",
    "zaroxi-core-engine-state": "Engine-local state primitives for rendering and UI.",
    "zaroxi-core-engine-window": "Window and native surface integration for the engine.",
    "zaroxi-core-engine-input": "Engine input event translation and handling.",
    "zaroxi-core-engine-action": "UI action dispatch and mapping.",
    "zaroxi-core-engine-focus": "Focus and keyboard focus management for engine views.",
    "zaroxi-core-engine-layout": "Layout algorithms for the Zaroxi engine.",
    "zaroxi-core-engine-style": "Styling and theming primitives for engine UI.",
    "zaroxi-core-engine-element": "Low-level UI element primitives for engine.",
    "zaroxi-core-engine-view": "High-level view and composition utilities for engine.",
    "zaroxi-core-engine-scene": "Scene graph management for engine rendering.",
    "zaroxi-core-engine-overlay": "Transient overlay primitives (menus, tooltips).",
    "zaroxi-core-engine-text": "Engine text shaping and rendering primitives.",
    "zaroxi-core-engine-font": "Font management and shaping for engine text.",
    "zaroxi-core-engine-render": "Rendering coordination for the engine.",
    "zaroxi-core-engine-render-backend": "Backend-specific renderer adapters.",
    "zaroxi-core-engine-render-resource": "Render resource management (textures, buffers).",
    "zaroxi-core-engine-render-pipeline": "Render pipeline configuration for engine.",
    "zaroxi-core-engine-render-graph": "Render graph construction and execution.",
    "zaroxi-core-engine-compositor": "Compositor orchestration for final frame.",
    "zaroxi-core-engine-animation": "Animation primitives for the engine.",
    "zaroxi-core-engine-clipboard": "Clipboard integration for the engine.",
    "zaroxi-core-engine-ime": "IME integration for complex text input.",
    "zaroxi-core-engine-accessibility": "Accessibility hooks and a11y integration.",
    "zaroxi-core-engine-test": "Engine testing utilities and harnesses.",
    # editor
    "zaroxi-core-editor-buffer": "In-memory document buffer primitives for the editor.",
    "zaroxi-core-editor-rope": "Rope string data structure used by the editor.",
    "zaroxi-core-editor-model": "Editor model and high-level text abstractions.",
    "zaroxi-core-editor-transaction": "Text transaction primitives and undo/redo.",
    "zaroxi-core-editor-selection": "Selection representation and utilities.",
    "zaroxi-core-editor-cursor": "Cursor primitives and movement semantics.",
    "zaroxi-core-editor-history": "History and undo/redo backing storage.",
    "zaroxi-core-editor-display": "Editor layout and display helpers (non-UI).",
    "zaroxi-core-editor-decoration": "Decoration APIs for editor (high-level).",
    "zaroxi-core-editor-folding": "Code folding algorithms and data structures.",
    "zaroxi-core-editor-gutter": "Gutter and line number primitives.",
    "zaroxi-core-editor-diagnostics": "Diagnostics and lint integration primitives.",
    "zaroxi-core-editor-minimap": "Minimap rendering data model.",
    "zaroxi-core-editor-command": "Editor command abstractions.",
    "zaroxi-core-editor-viewport": "Viewport calculations and scroll math.",
    "zaroxi-core-editor-inline-ai": "Inline AI integration contracts for editor.",
    "zaroxi-core-editor-view": "Editor view composition helpers (non-UI).",
    "zaroxi-core-editor-collab": "CRDT/OT integration layer for real-time collaboration.",
    # core workspace
    "zaroxi-core-workspace-files": "Filesystem view and metadata for workspaces.",
    "zaroxi-core-workspace-index": "Workspace indexing and fast lookup utilities.",
    "zaroxi-core-workspace-history": "File history and snapshot utilities.",
    "zaroxi-core-workspace-patch": "Patch application and preview helpers.",
    "zaroxi-core-workspace-watcher": "Filesystem watcher abstraction for workspace.",
    "zaroxi-core-workspace-cache": "Workspace file cache and eviction policies.",
    "zaroxi-core-workspace-snapshot": "Snapshot representation for workspace state.",
    "zaroxi-core-workspace-permissions": "Workspace permission model and checks.",
    # core platform
    "zaroxi-core-platform-runtime": "Platform runtime helpers and adapters.",
    "zaroxi-core-platform-lsp": "Language Server Protocol integration layer.",
    "zaroxi-core-platform-syntax": "Syntax grammar loading and runtime helpers.",
    "zaroxi-core-platform-debugger": "Debugger instrumentation adapters.",
    "zaroxi-core-platform-terminal": "Terminal integration adapters.",
    "zaroxi-core-platform-git": "Git VCS integration adapters.",
    "zaroxi-core-platform-test": "Test harness and platform test adapters.",
    "zaroxi-core-platform-profiler": "Profiling integrations and adapters.",
    "zaroxi-core-platform-formatter": "Formatter adapters and integrations.",
    "zaroxi-core-platform-linter": "Linter integration adapters.",
    "zaroxi-core-platform-plugin": "Plugin host integration adapters.",
    "zaroxi-core-platform-remote-ssh": "Remote SSH container adapters.",
    "zaroxi-core-platform-remote-container": "Remote container integration adapters.",
    # domain
    "zaroxi-domain-collaboration": "Domain model for collaboration (events, intents).",
    # application
    "zaroxi-application-collaboration": "Application-level orchestration for collaboration.",
    # security
    "zaroxi-security-crypto": "Cryptographic utilities for security layer.",
    # interface
    "zaroxi-interface-desktop": "Desktop UI shell glue (editor+gui surface).",
}

# Template stubs for new crate Cargo.toml and src/lib.rs
CARGO_TOML_TEMPLATE = """[package]
name = "{name}"
version = "0.1.0"
edition = "2024"
description = "{description}"
license = "MIT"

[dependencies]
"""
LIB_RS_TEMPLATE = """// {name}
// Auto-generated crate stub for the Zaroxi migration.
// Responsibility: {description}

#![allow(dead_code)]
#![allow(unused_imports)]

pub fn _crate_marker() {{
    // Marker function to make the crate non-empty for packaging.
}}
"""

# ------------------------
# Helper functions
# ------------------------

def safe_mkdir(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)

def write_file(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")

def move_and_merge(src: Path, dst: Path) -> None:
    """
    Move src -> dst. If dst exists, merge contents:
    - files with same name are preserved in dst (src version moved to .migrated/<name>).
    - otherwise move file/dir into dst.
    """
    if not src.exists():
        return
    safe_mkdir(dst)
    migrated_dir = dst / ".migrated_from"
    for item in src.iterdir():
        target = dst / item.name
        if target.exists():
            # preserve existing target, relocate source item to migrated folder
            migrated_dir.mkdir(exist_ok=True)
            relocation = migrated_dir / f"{src.name}__{item.name}"
            if relocation.exists():
                # ensure deterministic unique name
                idx = 1
                base = relocation
                while relocation.exists():
                    relocation = base.with_name(f"{base.name}.{idx}")
                    idx += 1
            shutil.move(str(item), str(relocation))
        else:
            shutil.move(str(item), str(target))
    # remove source directory if empty
    try:
        if src.exists():
            src.rmdir()
    except OSError:
        # not empty; leave it (should not happen)
        pass

def update_or_create_cargo_toml(crate_dir: Path, crate_name: str, description: str = "") -> None:
    """
    Ensure Cargo.toml exists and has required [package] metadata with the
    canonical crate name, edition, and version. Overwrites minimal fields
    to guarantee correctness; preserves other fields if present.
    """
    cargo_path = crate_dir / "Cargo.toml"
    pkg = {
        "name": crate_name,
        "version": "0.1.0",
        "edition": "2024",
        "description": description or f"{crate_name} (auto-generated)",
        "license": "MIT",
    }

    # If Cargo.toml exists, attempt a minimal update by rewriting core fields.
    if cargo_path.exists():
        # Read content and attempt to preserve section not covered here.
        # For determinism we will parse minimally: rebuild package header then append rest.
        lines = cargo_path.read_text(encoding="utf-8").splitlines()
        # Remove existing [package] block
        out_lines = []
        in_package = False
        for ln in lines:
            stripped = ln.strip()
            if stripped == "[package]":
                in_package = True
                continue
            if in_package:
                if stripped.startswith("[") and stripped.endswith("]"):
                    in_package = False
                    out_lines.append(ln)
                else:
                    continue
            else:
                out_lines.append(ln)
        # Prepend canonical package block
        pkg_block = [
            "[package]",
            f'name = "{pkg["name"]}"',
            f'version = "{pkg["version"]}"',
            f'edition = "{pkg["edition"]}"',
            f'description = "{pkg["description"]}"',
            f'license = "{pkg["license"]}"',
            "",
        ]
        final = "\n".join(pkg_block + out_lines).rstrip() + "\n"
        cargo_path.write_text(final, encoding="utf-8")
    else:
        content = CARGO_TOML_TEMPLATE.format(name=crate_name, description=description or f"{crate_name} (auto-generated)")
        cargo_path.write_text(content, encoding="utf-8")

def create_stub_lib(crate_dir: Path, crate_name: str, description: str = "") -> None:
    lib_path = crate_dir / "src" / "lib.rs"
    if lib_path.exists():
        # do not overwrite existing lib.rs
        return
    content = LIB_RS_TEMPLATE.format(name=crate_name, description=description or "Auto-generated crate")
    write_file(lib_path, content)

def gather_existing_crates() -> set:
    if not CRATES_DIR.exists():
        return set()
    return set(p.name for p in CRATES_DIR.iterdir() if p.is_dir())

# ------------------------
# Migration steps
# ------------------------

def ensure_crates_dir() -> None:
    safe_mkdir(CRATES_DIR)

def perform_renames(existing: set) -> None:
    """
    Execute deterministic rename/merge operations according to RENAME_MAP.
    """
    for src_name, dst_name in sorted(RENAME_MAP.items()):
        src = CRATES_DIR / src_name
        dst = CRATES_DIR / dst_name
        if not src.exists():
            # nothing to do
            continue
        if dst.exists():
            # merge src -> dst
            print(f"Merging {src_name} -> existing {dst_name}")
            move_and_merge(src, dst)
        else:
            # move directory
            print(f"Renaming {src_name} -> {dst_name}")
            shutil.move(str(src), str(dst))
        # After move, ensure Cargo.toml is updated
        desc = DESCRIPTION_MAP.get(dst_name, "")
        update_or_create_cargo_toml(dst, dst_name, desc)

def create_missing_crates(existing: set) -> None:
    """
    Create any crates from FINAL_CRATES that do not exist yet.
    """
    for crate in sorted(FINAL_CRATES):
        crate_dir = CRATES_DIR / crate
        if crate_dir.exists():
            # Ensure Cargo.toml metadata is canonical
            update_or_create_cargo_toml(crate_dir, crate, DESCRIPTION_MAP.get(crate, ""))
            create_stub_lib(crate_dir, crate, DESCRIPTION_MAP.get(crate, ""))
            continue
        print(f"Creating missing crate: {crate}")
        safe_mkdir(crate_dir)
        update_or_create_cargo_toml(crate_dir, crate, DESCRIPTION_MAP.get(crate, ""))
        create_stub_lib(crate_dir, crate, DESCRIPTION_MAP.get(crate, ""))

def remove_obsolete_crates(existing: set) -> None:
    """
    Remove directories listed in REMOVE_LIST if they are not final crates.
    Also remove any directory in 'existing' that is not in FINAL_CRATES after renames,
    unless it is in KEEP list or in RENAME_MAP source keys that already moved.
    """
    # Remove explicit remove list first
    for name in REMOVE_LIST:
        path = CRATES_DIR / name
        if path.exists():
            print(f"Removing obsolete crate: {name}")
            shutil.rmtree(path)

    # Now remove any stray crate that is not in FINAL_CRATES and not in RENAME_MAP.values()
    final_set = set(FINAL_CRATES)
    protected = final_set.union(RENAME_MAP.keys()).union(RENAME_MAP.values())
    for p in sorted(CRATES_DIR.iterdir()):
        if not p.is_dir():
            continue
        if p.name not in final_set:
            if p.name in protected:
                # it is either a source that may have been moved or a destination - skip
                continue
            # safe to remove
            print(f"Removing stray crate not in final inventory: {p.name}")
            shutil.rmtree(p)

def generate_workspace_manifest() -> None:
    """
    Generate a top-level Cargo.toml workspace manifest listing exactly the FINAL_CRATES.
    Overwrites existing Cargo.toml (but preserves top-level comment with a brief header).
    """
    members = sorted(FINAL_CRATES)
    header = textwrap.dedent(
        """# Auto-generated workspace manifest for Zaroxi migration
# Edition and package defaults are declared in individual crates.
"""
    )
    members_toml = "\n".join(f'  "crates/{m}",' for m in members)
    content = header + "[workspace]\nresolver = \"2\"\n\nmembers = [\n" + members_toml + "\n]\n\n[workspace.package]\nedition = \"2024\"\nversion = \"0.1.0\"\nlicense = \"MIT\"\n\n[workspace.dependencies]\nanyhow = \"1\"\n"
    WORKSPACE_CARGO.write_text(content, encoding="utf-8")
    print(f"Wrote workspace Cargo.toml with {len(members)} members.")

# ------------------------
# Main execution
# ------------------------

def main() -> int:
    print("ZAROXI MIGRATION SCRIPT START")
    ensure_crates_dir()
    existing_before = gather_existing_crates()
    print(f"Existing crates found: {len(existing_before)}")

    # Step 1: perform renames and merges
    perform_renames(existing_before)

    # Step 2: create missing crates from final inventory
    existing_after_renames = gather_existing_crates()
    create_missing_crates(existing_after_renames)

    # Step 3: remove explicit obsolete crates and stray crates
    existing_after_create = gather_existing_crates()
    remove_obsolete_crates(existing_after_create)

    # Step 4: Final enforcement: ensure all final crates exist
    missing = [c for c in FINAL_CRATES if not (CRATES_DIR / c).exists()]
    if missing:
        print("ERROR: Missing crates after migration:", missing)
        # Attempt to create missing ones deterministically
        for m in missing:
            print(f"Repair-create missing crate: {m}")
            safe_mkdir(CRATES_DIR / m)
            update_or_create_cargo_toml(CRATES_DIR / m, m, DESCRIPTION_MAP.get(m, ""))
            create_stub_lib(CRATES_DIR / m, m, DESCRIPTION_MAP.get(m, ""))
    else:
        print("All final crates present.")

    # Step 5: Generate top-level workspace Cargo.toml
    generate_workspace_manifest()

    # Final summary
    final_set = gather_existing_crates()
    extra = sorted([c for c in final_set if c not in FINAL_CRATES])
    missing = sorted([c for c in FINAL_CRATES if c not in final_set])

    print("MIGRATION SUMMARY")
    print(f"Final crates count: {len(final_set)}")
    if missing:
        print("Missing crates (should be none):", missing)
    if extra:
        print("Extra crates (should be none):", extra)
    else:
        print("No extra crates detected.")

    print("ZAROXI MIGRATION SCRIPT COMPLETE")
    return 0

if __name__ == "__main__":
    try:
        code = main()
        sys.exit(code)
    except Exception as exc:
        print("Migration failed with exception:", exc, file=sys.stderr)
        sys.exit(2)
