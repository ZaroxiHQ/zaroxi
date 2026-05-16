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
- Current crate inventory (grouped) with Purpose and Allowed deps
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

Layers and responsibilities (summary)

- Kernel (zaroxi-kernel-*)
  - Purpose: tiny, stable primitives with minimal dependencies (IDs, core types, small math, memory helpers, traits).
  - Allowed deps: standard library and a small set of vetted external crates. Kernel crates must not depend on other zaroxi-* crates.

- Core (zaroxi-core-*)
  - Purpose: foundational engines and primitives used by higher layers (rendering, text, input, runtime, scheduling).
  - Allowed deps: kernel crates and other core crates only.

- Domain (zaroxi-domain-*)
  - Purpose: domain models and pure business logic (workspace & project models, buffer semantics, editor domain logic).
  - Allowed deps: kernel and core crates, and other domain crates.

- Application (zaroxi-application-*)
  - Purpose: orchestration and feature composition built from domain + core primitives (editors, search, navigation, plugins).
  - Allowed deps: kernel, core, domain, and other application crates.

- Interface (zaroxi-interface-*)
  - Purpose: concrete UIs and entrypoints (desktop app, CLI, theming). Integrates application features into runnable products.
  - Allowed deps: application, domain, core, kernel, and other interface crates.

Special namespaces:
- Intelligence (zaroxi-intelligence-*)
  - Purpose: AI agents, planning, embeddings and related logic.
  - Allowed deps: kernel, core, domain only.

- Security (zaroxi-security-*)
  - Purpose: sandboxing, policy, validation, auth, audit, crypto primitives.
  - Allowed deps: kernel, core, domain only.

- Infrastructure (zaroxi-infrastructure-*)
  - Purpose: adapters and implementations for networking, storage, RPC, tracing, metrics, process management.
  - Allowed deps: kernel and core only.

---

Current crate inventory with Purpose and Allowed deps
(This section enumerates each crate present in the workspace and states a short Purpose and its Allowed deps.
Keep this section synchronized with Cargo.toml workspace.members. If you add/remove crates, update this list.)

APPLICATION
- zaroxi-application-ai
  - Purpose: High-level orchestration of AI features (copilots, assistant integrations, orchestration with intelligence-* tools).
  - Allowed deps: kernel, core, domain, application.

- zaroxi-application-collaboration
  - Purpose: Orchestrates collaboration features (session management, invitations, sync coordination at application level).
  - Allowed deps: kernel, core, domain, application.

- zaroxi-application-command
  - Purpose: Application-level command routing, permission checks, and command palette integration.
  - Allowed deps: kernel, core, domain, application.

- zaroxi-application-editor
  - Purpose: Compose domain and core features into the editor UX (feature toggles, glue code).
  - Allowed deps: kernel, core, domain, application.

- zaroxi-application-navigation
  - Purpose: Navigation services (go-to-definition, symbol indexes, cross-file navigation).
  - Allowed deps: kernel, core, domain, application.

- zaroxi-application-plugin
  - Purpose: Plugin management orchestration at application level (registration, lifecycle, permissions).
  - Allowed deps: kernel, core, domain, application, security (via trait interfaces).

- zaroxi-application-project
  - Purpose: Project-level orchestration (build/run integration, target provisioning).
  - Allowed deps: kernel, core, domain, application, infrastructure adapters via traits.

- zaroxi-application-refactor
  - Purpose: High-level refactoring workflows (preview, safe apply, multi-file changes).
  - Allowed deps: kernel, core, domain, application.

- zaroxi-application-remote
  - Purpose: Remote workspace orchestration and remote environment integration.
  - Allowed deps: kernel, core, domain, application, infrastructure-rpc (adapter).

- zaroxi-application-search
  - Purpose: Cross-file search orchestration and indexing consumers.
  - Allowed deps: kernel, core, domain, application.

- zaroxi-application-workspace
  - Purpose: Workspace-level orchestration (open/close, indexing triggers, workspace UI glue).
  - Allowed deps: kernel, core, domain, application.

CORE — Editor & UI
- zaroxi-core-commands
  - Purpose: Core command registry, execution model and undo/redo hooks.
  - Allowed deps: kernel, core.

- zaroxi-core-editor-buffer
  - Purpose: In-memory document buffer primitives for the editor (text storage, change application).
  - Allowed deps: kernel, core.

- zaroxi-core-editor-collab
  - Purpose: Low-level collaborative editing primitives and CRDT helpers (editor focus).
  - Allowed deps: kernel, core.

- zaroxi-core-editor-command
  - Purpose: Editor-specific command adapters and bindings for core-commands.
  - Allowed deps: kernel, core.

- zaroxi-core-editor-cursor
  - Purpose: Cursor movement model and low-level cursor helpers.
  - Allowed deps: kernel, core.

- zaroxi-core-editor-decoration
  - Purpose: Decoration model (spans, highlights) and rendering integration.
  - Allowed deps: kernel, core.

- zaroxi-core-editor-diagnostics
  - Purpose: Diagnostics collection and presentation plumbing for the editor.
  - Allowed deps: kernel, core, domain (via trait-only interfaces).

- zaroxi-core-editor-display
  - Purpose: Mapping buffer content to screen layout primitives.
  - Allowed deps: kernel, core.

- zaroxi-core-editor-folding
  - Purpose: Folding region detection and minimal runtime for fold state.
  - Allowed deps: kernel, core.

- zaroxi-core-editor-gutter
  - Purpose: Gutter rendering helpers and hit-testing.
  - Allowed deps: kernel, core.

- zaroxi-core-editor-history
  - Purpose: Low-level change history and checkpoint mechanics.
  - Allowed deps: kernel, core.

- zaroxi-core-editor-inline-ai
  - Purpose: Editor-side inline AI hinting integration (no network transport).
  - Allowed deps: kernel, core, domain (interfaces).

- zaroxi-core-editor-minimap
  - Purpose: Minimap rendering helpers and sampling utilities.
  - Allowed deps: kernel, core.

- zaroxi-core-editor-model
  - Purpose: Core model for editor views, buffer lenses, and projections.
  - Allowed deps: kernel, core.

- zaroxi-core-editor-rope
  - Purpose: Rope data structure optimized for large texts used by editor.
  - Allowed deps: kernel.

- zaroxi-core-editor-selection
  - Purpose: Selection sets and operations (multi-selection).
  - Allowed deps: kernel, core.

- zaroxi-core-editor-transaction
  - Purpose: Transaction semantics for batching edits and applying atomic changes.
  - Allowed deps: kernel, core.

- zaroxi-core-editor-view
  - Purpose: View-layer utilities for rendering and input coordination.
  - Allowed deps: kernel, core.

- zaroxi-core-editor-viewport
  - Purpose: Viewport management and virtualization helpers.
  - Allowed deps: kernel, core.

CORE — Engine, Runtime & Platform
- zaroxi-core-engine-accessibility
  - Purpose: Accessibility layer hooks and semantics for the engine.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-action
  - Purpose: Reusable action primitives used by UI and input subsystems.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-animation
  - Purpose: Animation utilities and timing helpers for UI elements.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-clipboard
  - Purpose: Clipboard abstraction used by engine and interface layers.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-compositor
  - Purpose: Frame composition and render pass orchestration.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-element
  - Purpose: Basic engine element primitives (widgets, nodes).
  - Allowed deps: kernel, core.

- zaroxi-core-engine-focus
  - Purpose: Focus management primitives for engine-driven UIs.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-font
  - Purpose: Font metrics, shaping helpers, and font atlas support.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-ime
  - Purpose: IME integration helpers and composition handling.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-input
  - Purpose: Normalized input event handling (keyboard/mouse/touch).
  - Allowed deps: kernel, core.

- zaroxi-core-engine-layout
  - Purpose: Layout engine primitives and measurement helpers.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-overlay
  - Purpose: Overlay management (popups, tooltips).
  - Allowed deps: kernel, core.

- zaroxi-core-engine-render
  - Purpose: Device-agnostic render APIs and resource abstractions.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-render-backend
  - Purpose: Backend-specific render adapters (wgpu, metal) behind feature flags.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-render-graph
  - Purpose: Render graph scheduling and resource dependency management.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-render-pipeline
  - Purpose: Pipeline description and shader binding utilities.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-render-resource
  - Purpose: GPU resource lifetime management and pooling.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-root
  - Purpose: Root application host primitives used by the engine.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-runtime
  - Purpose: Engine-side runtime primitives and runtime glue.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-scene
  - Purpose: Scene graph and compositing helpers.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-state
  - Purpose: Small-state and versioning utilities for engine components.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-style
  - Purpose: Styling and theme primitives used by engine & interface.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-test
  - Purpose: Engine test utilities and harnesses.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-text
  - Purpose: Text shaping glue and editor text integration.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-view
  - Purpose: View composition utilities and high-level view constructs.
  - Allowed deps: kernel, core.

- zaroxi-core-engine-window
  - Purpose: Windowing abstractions and surface management (platform adapters live outside).
  - Allowed deps: kernel, core.

- zaroxi-core-event
  - Purpose: Event bus and typed event propagation utilities.
  - Allowed deps: kernel, core.

- zaroxi-core-input
  - Purpose: Low-level input devices & adapters.
  - Allowed deps: kernel, core.

- zaroxi-core-io
  - Purpose: Low-level IO abstractions used by core (non-platform specific).
  - Allowed deps: kernel, core.

- zaroxi-core-plugin-runtime
  - Purpose: Plugin runtime contracts consumed by core plugin host.
  - Allowed deps: kernel, core, security (interfaces).

- zaroxi-core-runtime
  - Purpose: Runtime helpers and small schedulers used by core.
  - Allowed deps: kernel, core.

- zaroxi-core-scheduler
  - Purpose: Scheduling primitives for short-lived tasks.
  - Allowed deps: kernel, core.

- zaroxi-core-state
  - Purpose: Small state containers and versioning.
  - Allowed deps: kernel, core.

- zaroxi-core-sync
  - Purpose: Synchronization primitives and adapters.
  - Allowed deps: kernel, core.

- zaroxi-core-task
  - Purpose: Task abstraction used by runtime and scheduler.
  - Allowed deps: kernel, core.

- zaroxi-core-telemetry
  - Purpose: Minimal telemetry hooks for core components.
  - Allowed deps: kernel, core, infrastructure (for adapters).

- zaroxi-core-threading
  - Purpose: Threading primitives and pools.
  - Allowed deps: kernel, core.

CORE — Platform utilities
- zaroxi-core-platform-debugger
  - Purpose: Debug protocol helpers and adapters (trait-only).
  - Allowed deps: kernel, core.

- zaroxi-core-platform-formatter
  - Purpose: Formatting engine adapters (pluggable).
  - Allowed deps: kernel, core, domain (interfaces).

- zaroxi-core-platform-git
  - Purpose: Git model and trait interfaces for platform integrations.
  - Allowed deps: kernel, core.

- zaroxi-core-platform-linter
  - Purpose: Linter adapters and rule integration points.
  - Allowed deps: kernel, core, domain (interfaces).

- zaroxi-core-platform-lsp
  - Purpose: LSP integration primitives and protocol helpers.
  - Allowed deps: kernel, core, domain (interfaces).

- zaroxi-core-platform-plugin
  - Purpose: Platform-level plugin host contracts and lightweight adapters.
  - Allowed deps: kernel, core, security (interfaces).

- zaroxi-core-platform-profiler
  - Purpose: Profiling hooks and sampling adapters.
  - Allowed deps: kernel, core.

- zaroxi-core-platform-remote-container
  - Purpose: Remote container runtime adapters (trait definitions).
  - Allowed deps: kernel, core.

- zaroxi-core-platform-remote-ssh
  - Purpose: SSH remote access adapters (trait definitions).
  - Allowed deps: kernel, core.

- zaroxi-core-platform-runtime
  - Purpose: Platform runtime adapters and shims.
  - Allowed deps: kernel, core.

- zaroxi-core-platform-syntax
  - Purpose: Syntax/grammar registry and runtime loaders (Tree-sitter adapters).
  - Allowed deps: kernel, core, domain (interfaces).

- zaroxi-core-platform-terminal
  - Purpose: Terminal integration primitives.
  - Allowed deps: kernel, core.

- zaroxi-core-platform-test
  - Purpose: Testing harnesses and adapters.
  - Allowed deps: kernel, core.

CORE — Workspace helpers
- zaroxi-core-workspace-cache
  - Purpose: In-memory caching primitives for workspace uses.
  - Allowed deps: kernel, core.

- zaroxi-core-workspace-files
  - Purpose: Workspace file model primitives (no IO).
  - Allowed deps: kernel, core, domain (interfaces).

- zaroxi-core-workspace-history
  - Purpose: Workspace-level history and timeline utilities.
  - Allowed deps: kernel, core.

- zaroxi-core-workspace-index
  - Purpose: Index model primitives and metadata structures.
  - Allowed deps: kernel, core, domain (interfaces).

- zaroxi-core-workspace-patch
  - Purpose: Patch application model and preview logic.
  - Allowed deps: kernel, core, domain (interfaces).

- zaroxi-core-workspace-permissions
  - Purpose: Permission check primitives for workspace operations.
  - Allowed deps: kernel, core, security (interfaces).

- zaroxi-core-workspace-snapshot
  - Purpose: Workspace snapshotting primitives.
  - Allowed deps: kernel, core.

- zaroxi-core-workspace-watcher
  - Purpose: Watcher trait definitions; platform-specific implementations live in infrastructure.
  - Allowed deps: kernel, core.

DOMAIN
- zaroxi-domain-workspace
  - Purpose: Workspace model (projects, folders, metadata).
  - Allowed deps: kernel, core, domain.

- zaroxi-domain-project
  - Purpose: Project model, dependency graph, target metadata.
  - Allowed deps: kernel, core, domain.

- zaroxi-domain-settings
  - Purpose: Canonical settings model and typed schema.
  - Allowed deps: kernel, core, domain.

- zaroxi-domain-session
  - Purpose: Session lifecycle, user/session-scoped state.
  - Allowed deps: kernel, core, domain.

- zaroxi-domain-ai
  - Purpose: AI context models, prompt packing and domain-specific prompt logic.
  - Allowed deps: kernel, core, domain.

- zaroxi-domain-plugin
  - Purpose: Domain-level plugin contracts and behavior models.
  - Allowed deps: kernel, core, domain, security (interfaces).

- zaroxi-domain-buffer
  - Purpose: High-level buffer semantics that sit above core-editor-buffer.
  - Allowed deps: kernel, core, domain.

- zaroxi-domain-collaboration
  - Purpose: Domain-level collaboration models and policies.
  - Allowed deps: kernel, core, domain.

INFRASTRUCTURE
- zaroxi-infrastructure-container
  - Purpose: Container orchestration and adapter implementations (remote execution).
  - Allowed deps: kernel, core.

- zaroxi-infrastructure-http
  - Purpose: HTTP adapters and servers used by infra services.
  - Allowed deps: kernel, core.

- zaroxi-infrastructure-logging
  - Purpose: Logging sinks and adapters (OTel, file).
  - Allowed deps: kernel, core.

- zaroxi-infrastructure-metrics
  - Purpose: Metrics exporters and collectors.
  - Allowed deps: kernel, core.

- zaroxi-infrastructure-network
  - Purpose: Network client/server adapters and utilities.
  - Allowed deps: kernel, core.

- zaroxi-infrastructure-permissions
  - Purpose: Adapter implementations for permission persistence and enforcement.
  - Allowed deps: kernel, core, security (interfaces).

- zaroxi-infrastructure-process
  - Purpose: Process spawning and sandboxed process management.
  - Allowed deps: kernel, core.

- zaroxi-infrastructure-rpc
  - Purpose: RPC transport implementations (JSON-RPC / gRPC adapters).
  - Allowed deps: kernel, core.

- zaroxi-infrastructure-settings
  - Purpose: Settings persistence implementations.
  - Allowed deps: kernel, core.

- zaroxi-infrastructure-ssh
  - Purpose: SSH transport adapters.
  - Allowed deps: kernel, core.

- zaroxi-infrastructure-storage
  - Purpose: Storage adapters (local FS, S3-like backends).
  - Allowed deps: kernel, core.

- zaroxi-infrastructure-tracing
  - Purpose: Tracing adapters and exporters.
  - Allowed deps: kernel, core.

INTELLIGENCE
- zaroxi-intelligence-agent
  - Purpose: Agent runtime and orchestrator for intelligent features (no direct outbound network).
  - Allowed deps: kernel, core, domain.

- zaroxi-intelligence-planning
  - Purpose: Planning algorithms and plan representation.
  - Allowed deps: kernel, core, domain.

- zaroxi-intelligence-memory
  - Purpose: In-memory memory caches and vector store primitives.
  - Allowed deps: kernel, core, domain.

- zaroxi-intelligence-context
  - Purpose: Context packing, token budgeting, prompt construction utilities.
  - Allowed deps: kernel, core, domain.

- zaroxi-intelligence-tools
  - Purpose: Tool contract definitions used by agents (filesystem, git, etc.) — implementations live in infra.
  - Allowed deps: kernel, core, domain.

- zaroxi-intelligence-orchestrator
  - Purpose: Multi-agent coordination and orchestration.
  - Allowed deps: kernel, core, domain, security (interfaces).

- zaroxi-intelligence-eval
  - Purpose: Evaluation harnesses for agent outputs and metrics.
  - Allowed deps: kernel, core, domain.

- zaroxi-intelligence-embedding
  - Purpose: Embedding utilities and vector helpers (no external network).
  - Allowed deps: kernel, core, domain.

SECURITY
- zaroxi-security-sandbox
  - Purpose: Sandbox implementations and isolation primitives.
  - Allowed deps: kernel, core.

- zaroxi-security-policy
  - Purpose: Policy language and evaluation engine.
  - Allowed deps: kernel, core, domain (interfaces).

- zaroxi-security-validation
  - Purpose: Artifact validation and integrity checks.
  - Allowed deps: kernel, core.

- zaroxi-security-auth
  - Purpose: Authentication flows and token validation.
  - Allowed deps: kernel, core, infrastructure (for adapters).

- zaroxi-security-audit
  - Purpose: Structured audit event model and persistence helpers.
  - Allowed deps: kernel, core, infrastructure (for adapters).

- zaroxi-security-crypto
  - Purpose: Cryptographic primitives and small wrappers (key handling).
  - Allowed deps: kernel, core.

INTERFACE
- zaroxi-interface-app
  - Purpose: Desktop/web application shell and orchestration (entrypoint).
  - Allowed deps: kernel, core, domain, application, interface.

- zaroxi-interface-desktop
  - Purpose: Desktop-specific UI integration (native windowing, menus).
  - Allowed deps: kernel, core, application, interface.

- zaroxi-interface-cli
  - Purpose: CLI entrypoints and headless operations.
  - Allowed deps: kernel, core, application, interface.

- zaroxi-interface-theme
  - Purpose: Theme definitions and styling assets used by interface layers.
  - Allowed deps: kernel, core, interface.

KERNEL
- zaroxi-kernel-core
  - Purpose: Core minimal utilities and shared low-level helpers.
  - Allowed deps: standard library, vetted externals.

- zaroxi-kernel-types
  - Purpose: Canonical type definitions (Id, Position, Range).
  - Allowed deps: zaroxi-kernel-core, std.

- zaroxi-kernel-errors
  - Purpose: Centralized error types and conversion helpers.
  - Allowed deps: zaroxi-kernel-core, zaroxi-kernel-types.

- zaroxi-kernel-memory
  - Purpose: Allocation helpers and small pool implementations.
  - Allowed deps: zaroxi-kernel-core, zaroxi-kernel-types.

- zaroxi-kernel-async
  - Purpose: Small async utilities and primitives (no runtime).
  - Allowed deps: zaroxi-kernel-core.

- zaroxi-kernel-time
  - Purpose: Time and monotonic clock wrappers.
  - Allowed deps: zaroxi-kernel-core.

- zaroxi-kernel-math
  - Purpose: Geometry and numeric helpers.
  - Allowed deps: zaroxi-kernel-core.

- zaroxi-kernel-collections
  - Purpose: Specialized collection types used across the workspace.
  - Allowed deps: zaroxi-kernel-core.

- zaroxi-kernel-traits
  - Purpose: Minimal trait definitions used by multiple layers.
  - Allowed deps: zaroxi-kernel-core.

- zaroxi-kernel-config
  - Purpose: Canonical configuration schema types (no IO).
  - Allowed deps: zaroxi-kernel-core, zaroxi-kernel-types.

- zaroxi-kernel-protocol
  - Purpose: Small protocol-level types and shared wire formats.
  - Allowed deps: zaroxi-kernel-core, zaroxi-kernel-types.

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
