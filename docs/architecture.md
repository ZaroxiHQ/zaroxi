# Zaroxi Architecture — Professional Specification

This document is the canonical architecture specification for Zaroxi. It reflects the current workspace layout,
captures responsibilities for each layer and crate, and documents the enforcement and contribution process so
maintainers and contributors can act consistently and safely.

Quick summary
- Layers (top → bottom): interface → application → domain → core → kernel
- Special namespaces: intelligence, security, infrastructure (each has restricted dependencies)
- Enforcement: CI + repository scripts (naming, layer deps, cycle detection, size) ensure no upward/cross-layer
  dependencies or cycles are introduced.

Table of contents
- Overview & diagram
- Layers and responsibilities
- Current crate inventory (grouped)
- Naming & dependency rules (concise)
- Enforcement & CI (what tools do)
- Onboarding: adding a crate
- When and how to split crates
- Glossary & contact

---

Overview (diagram)
interface
  ↑
application
  ↑
domain
  ↑
core
  ↑
kernel

Special: intelligence, security, infrastructure (see rules below)

---

Layers and responsibilities

- Kernel (zaroxi-kernel-*)
  - Purpose: tiny, stable primitives with minimal dependencies. Types, IDs, small math, memory helpers, trait definitions.
  - Rule: Kernel crates must not depend on other zaroxi-* crates (only standard library and vetted external crates).

- Core (zaroxi-core-*)
  - Purpose: fundamental engines and primitives used by higher layers: rendering, text, input, runtime, scheduling.
  - Rule: Core may depend on kernel and other core crates only.

- Domain (zaroxi-domain-*)
  - Purpose: domain models and pure business logic: workspace model, buffer semantics, editor domain logic.
  - Rule: Domain may depend on kernel and core and other domain crates only.

- Application (zaroxi-application-*)
  - Purpose: orchestration and feature composition built from domain + core primitives (editors, search, navigation).
  - Rule: Application may depend on kernel, core, domain, and other application crates.

- Interface (zaroxi-interface-*)
  - Purpose: concrete UIs and entrypoints (desktop app, CLI); may depend on everything below.
  - Rule: Interface may depend on application, domain, core, kernel, and interface crates.

Special namespaces:
- Intelligence (zaroxi-intelligence-*)
  - May depend on kernel, core, domain. No application/interface/infrastructure deps.
- Security (zaroxi-security-*)
  - May depend on kernel, core, domain. No higher-level deps.
- Infrastructure (zaroxi-infrastructure-*)
  - Adapter layer: may depend on kernel and core only. Must not depend on domain/application/interface.

---

Current crate inventory (grouped by layer)
(Inventory taken from the workspace crates/ directory; keep this list current.)

Application
- zaroxi-application-ai
- zaroxi-application-collaboration
- zaroxi-application-command
- zaroxi-application-editor
- zaroxi-application-navigation
- zaroxi-application-plugin
- zaroxi-application-project
- zaroxi-application-refactor
- zaroxi-application-remote
- zaroxi-application-search
- zaroxi-application-workspace

Core — Editor & UI (core editor primitives and features)
- zaroxi-core-commands
- zaroxi-core-editor-buffer
- zaroxi-core-editor-collab
- zaroxi-core-editor-command
- zaroxi-core-editor-cursor
- zaroxi-core-editor-decoration
- zaroxi-core-editor-diagnostics
- zaroxi-core-editor-display
- zaroxi-core-editor-folding
- zaroxi-core-editor-gutter
- zaroxi-core-editor-history
- zaroxi-core-editor-inline-ai
- zaroxi-core-editor-minimap
- zaroxi-core-editor-model
- zaroxi-core-editor-rope
- zaroxi-core-editor-selection
- zaroxi-core-editor-transaction
- zaroxi-core-editor-view
- zaroxi-core-editor-viewport

Core — Engine, Runtime & Platform
- zaroxi-core-engine-accessibility
- zaroxi-core-engine-action
- zaroxi-core-engine-animation
- zaroxi-core-engine-clipboard
- zaroxi-core-engine-compositor
- zaroxi-core-engine-element
- zaroxi-core-engine-focus
- zaroxi-core-engine-font
- zaroxi-core-engine-ime
- zaroxi-core-engine-input
- zaroxi-core-engine-layout
- zaroxi-core-engine-overlay
- zaroxi-core-engine-render
- zaroxi-core-engine-render-backend
- zaroxi-core-engine-render-graph
- zaroxi-core-engine-render-pipeline
- zaroxi-core-engine-render-resource
- zaroxi-core-engine-root
- zaroxi-core-engine-runtime
- zaroxi-core-engine-scene
- zaroxi-core-engine-state
- zaroxi-core-engine-style
- zaroxi-core-engine-test
- zaroxi-core-engine-text
- zaroxi-core-engine-view
- zaroxi-core-engine-window
- zaroxi-core-event
- zaroxi-core-input
- zaroxi-core-io
- zaroxi-core-plugin-runtime
- zaroxi-core-runtime
- zaroxi-core-scheduler
- zaroxi-core-state
- zaroxi-core-sync
- zaroxi-core-task
- zaroxi-core-telemetry
- zaroxi-core-threading

Core — Platform utilities
- zaroxi-core-platform-debugger
- zaroxi-core-platform-formatter
- zaroxi-core-platform-git
- zaroxi-core-platform-linter
- zaroxi-core-platform-lsp
- zaroxi-core-platform-plugin
- zaroxi-core-platform-profiler
- zaroxi-core-platform-remote-container
- zaroxi-core-platform-remote-ssh
- zaroxi-core-platform-runtime
- zaroxi-core-platform-syntax
- zaroxi-core-platform-terminal
- zaroxi-core-platform-test

Core — Workspace helpers
- zaroxi-core-workspace-cache
- zaroxi-core-workspace-files
- zaroxi-core-workspace-history
- zaroxi-core-workspace-index
- zaroxi-core-workspace-patch
- zaroxi-core-workspace-permissions
- zaroxi-core-workspace-snapshot
- zaroxi-core-workspace-watcher

Domain
- zaroxi-domain-ai
- zaroxi-domain-buffer
- zaroxi-domain-collaboration
- zaroxi-domain-plugin
- zaroxi-domain-project
- zaroxi-domain-session
- zaroxi-domain-settings
- zaroxi-domain-workspace

Infrastructure
- zaroxi-infrastructure-container
- zaroxi-infrastructure-http
- zaroxi-infrastructure-logging
- zaroxi-infrastructure-metrics
- zaroxi-infrastructure-network
- zaroxi-infrastructure-permissions
- zaroxi-infrastructure-process
- zaroxi-infrastructure-rpc
- zaroxi-infrastructure-settings
- zaroxi-infrastructure-ssh
- zaroxi-infrastructure-storage
- zaroxi-infrastructure-tracing

Intelligence
- zaroxi-intelligence-agent
- zaroxi-intelligence-context
- zaroxi-intelligence-embedding
- zaroxi-intelligence-eval
- zaroxi-intelligence-memory
- zaroxi-intelligence-orchestrator
- zaroxi-intelligence-planning
- zaroxi-intelligence-safety
- zaroxi-intelligence-tools

Interface
- zaroxi-interface-app
- zaroxi-interface-cli
- zaroxi-interface-desktop
- zaroxi-interface-theme

Kernel
- zaroxi-kernel-async
- zaroxi-kernel-collections
- zaroxi-kernel-config
- zaroxi-kernel-core
- zaroxi-kernel-errors
- zaroxi-kernel-math
- zaroxi-kernel-memory
- zaroxi-kernel-protocol
- zaroxi-kernel-time
- zaroxi-kernel-traits
- zaroxi-kernel-types

Security
- zaroxi-security-audit
- zaroxi-security-auth
- zaroxi-security-crypto
- zaroxi-security-policy
- zaroxi-security-sandbox
- zaroxi-security-validation

Note: keep this inventory in sync with Cargo.toml workspace members. Add/remove crates there first.

---

Naming & dependency rules (concise)

- Crate naming:
  - Must begin with zaroxi-.
  - Format: zaroxi-{layer}-{...}. Core crates are encouraged to use sublayer tokens (e.g. zaroxi-core-engine-*, zaroxi-core-editor-*).
  - Use check_crate_naming.py to validate names.

- Dependency directions (strict):
  - kernel → kernel | external
  - core → kernel | core
  - domain → kernel | core | domain
  - application → kernel | core | domain | application
  - interface → kernel | core | domain | application | interface
  - intelligence → kernel | core | domain
  - security → kernel | core | domain
  - infrastructure → kernel | core

- Forbidden:
  - No crate may depend on a higher-level layer (e.g., core → domain).
  - No cycles.
  - Infrastructure must not depend on domain/application/interface.
  - Intelligence and security must not depend on application/interface/infrastructure.

Reasoning: Clear upward boundaries keep the core small, testable, and stable. If cross-layer access is required, extract a facade into an allowed layer.

---

Enforcement & CI

Scripts (in .github/scripts/)
- check_crate_naming.py — validates crate names and duplicate concerns.
- check_layer_deps.py — enforces the layer matrix; supports --report and --fix-suggestions.
- check_circular_deps.py — detects cycles in the internal dependency graph.
- check_crate_size.py — counts Rust source lines per crate and alerts on large crates.

GitHub Actions
- CI runs formatting, clippy, tests, cargo-deny, and the architecture scripts.
- The layer-deps job produces a JSON report artifact for maintainers to review.
- Configure branch protection to require these checks before merging.

Local workflow (recommended)
- just arch — run architecture checks locally (naming, layer deps, cycles, size).
- just arch-report — produce a JSON report and view with jq.
- just size — run only the size check.
- just validate — run arch + deny + fmt + clippy (pre-push gate).

---

Onboarding: adding a crate

1. Pick a crate name following naming rules.
2. Decide the correct layer; prefer placing generic/shared code into core/domain rather than elevating dependencies.
3. Use the helper: just new-crate zaroxi-{layer}-{name}.
4. Add the crate directory and ensure Cargo.toml is correct.
5. Update root Cargo.toml workspace.members if the helper doesn't already.
6. Run local checks:
   - just arch
   - just check
   - just test
   - just deny

If check_layer_deps reports a violation, follow the suggestion: either move functionality to a lower layer crate or refactor the consumer.

---

When to split a crate

- Size heuristic: if a crate grows beyond ~1,500 lines of Rust, consider splitting; CI will warn at 1,500 and fail at 3,000.
- Responsibility split: separate UI/view logic from pure domain models and from adapters.
- Dependency pressure: if a crate requires dependencies that violate layer rules, split boundaries to restore the allowed graph.
- API surface: prefer small, well-documented public facades with internal helper crates marked as internal.

---

Glossary & contact

- Facade: a small stable crate that provides a focused public API other crates can depend on.
- Adapter: an infrastructure implementation of a trait defined in core/domain (lives in infrastructure-* or platform-*).
- Maintainers: open an issue and mention the architecture leads for exceptions or naming disputes.

---

Recommended next steps for maintainers

- Ensure Cargo.toml workspace.members reflect the inventory above.
- Run the architecture scripts and fix violations incrementally.
- Add small shim crates if needed to preserve compatibility while splitting monoliths.
- Keep docs/architecture.md in sync when creating or removing crates.

---

End of specification.
