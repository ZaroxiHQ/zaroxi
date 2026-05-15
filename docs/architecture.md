# Zaroxi — Complete Architecture Specification (v1.0)

Authoritative, unambiguous, low-level + high-level architecture for a production-grade, AI-first, GPU-accelerated IDE.

---

## Short summary of immediate changes (3 steps)

1. Establish a kernel-\* foundation: create zero-dependency crates that everything else depends on; move primitive IDs, time, memory,
   minimal traits into these crates.
2. Reorganize existing crates according to the full rename map; create new crate stubs for any missing pieces and split monoliths
   (engine → core-\*).
3. Enforce strict dependency rules via workspace Cargo.toml, crate-level CI checks, and compile-time features (deny unsafe
   cross-layer deps, lints + crate-bounds tests).

---

## 1. Final crate tree (top-down by layer)

Note: crates appear as `<group>-<name>` and correspond to Rust crate directories in the workspace.

1. kernel-\*
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

2. core-\* (low-level engines)
   - zaroxi-core-input
   - zaroxi-core-event
   - zaroxi-core-commands
   - zaroxi-core-runtime
   - zaroxi-core-scheduler
   - zaroxi-core-task
   - zaroxi-core-threading
   - zaroxi-core-text
   - zaroxi-core-text-buffer
   - zaroxi-core-text-rope
   - zaroxi-core-text-edit
   - zaroxi-core-text-diff
   - zaroxi-core-render
   - zaroxi-core-render-backend
   - zaroxi-core-render-graph
   - zaroxi-core-render-pipeline
   - zaroxi-core-render-resource
   - zaroxi-core-render-text
   - zaroxi-core-render-ui
   - zaroxi-core-render-scene
   - zaroxi-core-render-compositor
   - zaroxi-core-layout
   - zaroxi-core-ui

3. domain-\*
   - zaroxi-domain-editor
   - zaroxi-domain-workspace
   - zaroxi-domain-project
   - zaroxi-domain-buffer
   - zaroxi-domain-selection
   - zaroxi-domain-cursor
   - zaroxi-domain-history
   - zaroxi-domain-ai
   - zaroxi-domain-settings

4. application-\*
   - zaroxi-application-editor
   - zaroxi-application-workspace
   - zaroxi-application-project
   - zaroxi-application-buffer
   - zaroxi-application-command
   - zaroxi-application-search
   - zaroxi-application-ai
   - zaroxi-application-navigation
   - zaroxi-application-refactor

5. intelligence-\*
   - zaroxi-intelligence-agent
   - zaroxi-intelligence-planning
   - zaroxi-intelligence-memory
   - zaroxi-intelligence-context
   - zaroxi-intelligence-tools
   - zaroxi-intelligence-orchestrator
   - zaroxi-intelligence-eval
   - zaroxi-intelligence-embedding

6. platform-\*
   - zaroxi-platform-lsp
   - zaroxi-platform-syntax
   - zaroxi-platform-debugger
   - zaroxi-platform-terminal
   - zaroxi-platform-git
   - zaroxi-platform-test
   - zaroxi-platform-profiler
   - zaroxi-platform-formatter
   - zaroxi-platform-linter
   - zaroxi-platform-plugin

7. workspace-\*
   - zaroxi-workspace-files
   - zaroxi-workspace-index
   - zaroxi-workspace-history
   - zaroxi-workspace-patch
   - zaroxi-workspace-watcher
   - zaroxi-workspace-cache

8. collaboration-\*
   - zaroxi-collaboration-sync
   - zaroxi-collaboration-crdt
   - zaroxi-collaboration-presence
   - zaroxi-collaboration-session

9. infrastructure-\*
   - zaroxi-infrastructure-rpc
   - zaroxi-infrastructure-http
   - zaroxi-infrastructure-storage
   - zaroxi-infrastructure-settings
   - zaroxi-infrastructure-permissions
   - zaroxi-infrastructure-logging
   - zaroxi-infrastructure-metrics
   - zaroxi-infrastructure-tracing

10. security-\*

- zaroxi-security-sandbox
- zaroxi-security-policy
- zaroxi-security-validation
- zaroxi-security-auth

11. interface-\*

- zaroxi-interface-app
- zaroxi-interface-editor
- zaroxi-interface-theme
- zaroxi-interface-cli
- zaroxi-interface-gui

Tooling & infra (workspace-level)

- workspace/Cargo.toml
- tools/crate-lint (dependency rule checks)
- tools/ci (CI pipeline scripts)
- docs/ (this file)

---

## 2. Renamed crates list (strict mapping)

Apply exactly these renames to transform the existing repository into the new layout:

- zaroxi-engine-_ → zaroxi-core-_
- zaroxi-service-_ → zaroxi-application-_
- zaroxi-infra-_ → zaroxi-infrastructure-_
- zaroxi-ai-agent → zaroxi-intelligence-agent
- zaroxi-domain-ai-context → zaroxi-domain-ai
- zaroxi-lang-_ → zaroxi-platform-_
- zaroxi-ops-_ → zaroxi-workspace-_
- zaroxi-foundation → zaroxi-kernel-core
- zaroxi-app → zaroxi-interface-app
- zaroxi-editor-\* → split into:
  - zaroxi-interface-editor
  - zaroxi-core-text
  - zaroxi-domain-buffer

Additional strict rename rules:

- Any crate that currently mixes UI + domain + infra must be split into one of the above layers; see "Removed/merged crates".

---

## 3. New crates to create (stubs first, then full impl)

(These are minimal-first-to-latear build order.)

kernel layer:

- zaroxi-kernel-memory
- zaroxi-kernel-async
- zaroxi-kernel-collections
- zaroxi-kernel-traits

core layer:

- zaroxi-core-render-graph
- zaroxi-core-render-compositor
- zaroxi-core-text-rope (if not present)
- zaroxi-core-text-diff

intelligence:

- zaroxi-intelligence-orchestrator
- zaroxi-intelligence-embedding

platform:

- zaroxi-platform-plugin (plugin host API + sandbox adapters)

workspace:

- zaroxi-workspace-index
- zaroxi-workspace-cache

infrastructure:

- zaroxi-infrastructure-tracing (OpenTelemetry adapter)
- zaroxi-infrastructure-metrics (Prometheus/OTel)

security:

- zaroxi-security-sandbox
- zaroxi-security-policy

interface:

- zaroxi-interface-editor (desktop/web entry points)

tools:

- tools/crate-lint
- tools/generate-workspace (script to create stubs and enforce names)

---

## 4. Removed / merged crates (monolith breakup)

- Remove the monolithic zaroxi-engine crate. Its responsibilities are redistributed to zaroxi-core-\* crates.
- Merge zaroxi-foundation → zaroxi-kernel-core (keeps low-level ids and helpers).
- Split zaroxi-app → zaroxi-interface-app (UI shell) + domain pieces (moved into domain-\* crates).
- Any crate that currently mixes platform (language parsing) with runtime or UI must be split into platform-_ and core-_ crates
  respectively.

---

## 5. Dependency graph (concise, deterministic)

Rules:

- Allowed topological flow: interface → application → domain → core → kernel
- application → intelligence
- application → platform
- application → collaboration
- Only kernel crates depend on no zaroxi crates.
- core and domain may depend on kernel only.
- infrastructure crates implement adapters and depend on kernel and core traits but not domain or application.

High-level graph (edges read as "→ depends on"):

- zaroxi-interface-app → zaroxi-application-editor
- zaroxi-application-editor → zaroxi-domain-editor, zaroxi-application-command, zaroxi-application-ai
- zaroxi-domain-editor → zaroxi-domain-buffer, zaroxi-domain-selection, zaroxi-domain-history
- zaroxi-domain-buffer → zaroxi-core-text, zaroxi-kernel-collections
- zaroxi-core-text → zaroxi-kernel-\* (core primitives)
- zaroxi-core-render → zaroxi-core-render-backend, zaroxi-core-render-text, zaroxi-kernel-\*
- zaroxi-intelligence-orchestrator → zaroxi-intelligence-agent, zaroxi-intelligence-planning, zaroxi-infrastructure-rpc (adapter)
- zaroxi-platform-lsp → zaroxi-core-runtime, zaroxi-infrastructure-rpc
- zaroxi-collaboration-session → zaroxi-collaboration-crdt, zaroxi-infrastructure-rpc
- zaroxi-security-sandbox → zaroxi-infrastructure-storage (sandboxed adapter), zaroxi-kernel-traits

Enforcement:

- Workspace-level CI contains a crate graph verification tool (tools/crate-lint) that parses Cargo.toml and ensures no forbidden
  edges exist. Failing the CI blocks merges.

---

## 6. Per-crate: purpose and allowed dependencies (full list)

Rules: "allowed dependencies" lists what the crate may depend on (project-local crates + external crates). External crates are
examples; choices left open but must be vetted.

--- Kernel layer (NO zaroxi deps allowed)

- zaroxi-kernel-core
  - Purpose: foundational helpers, Result/ResultExt, small macros, minimal types that are safe to re-export.
  - Allowed deps: std only.
- zaroxi-kernel-types
  - Purpose: Ids (Uuid wrappers), Position, Span, Range, small serializable primitives.
  - Allowed deps: zaroxi-kernel-core, std, no zaroxi-\* crates.
- zaroxi-kernel-errors
  - Purpose: unified error types and conversion utilities, Error trait wrappers.
  - Allowed deps: zaroxi-kernel-core, zaroxi-kernel-types.
- zaroxi-kernel-memory
  - Purpose: arena allocators, pool allocators, bump allocations, smallvec-like pools.
  - Allowed deps: zaroxi-kernel-core, zaroxi-kernel-types.
- zaroxi-kernel-async
  - Purpose: tiny task primitives (sync primitives, no runtime), futures helpers.
  - Allowed deps: zaroxi-kernel-core.
- zaroxi-kernel-time
  - Purpose: monotonic time wrapper, timers used by core layers.
  - Allowed deps: zaroxi-kernel-core.
- zaroxi-kernel-math
  - Purpose: geometry, viewport math, layout helpers.
  - Allowed deps: zaroxi-kernel-core.
- zaroxi-kernel-collections
  - Purpose: lock-free containers where possible, compact maps, safe wrappers around atomic structures.
  - Allowed deps: zaroxi-kernel-core.
- zaroxi-kernel-traits
  - Purpose: minimal trait definitions referenced across the system (persist, serialize, id traits).
  - Allowed deps: zaroxi-kernel-core.
- zaroxi-kernel-config
  - Purpose: canonical config schema types used by infra & app (no IO).
  - Allowed deps: zaroxi-kernel-core, zaroxi-kernel-types.

--- Core layer (only depend on kernel)

- zaroxi-core-input
  - Purpose: normalized input events, keyboard/mouse/pen, raw event queue.
  - Allowed deps: zaroxi-kernel-\*, small external crates (winit only in backend adapters).
- zaroxi-core-event
  - Purpose: event bus (lock-free where possible), typed event propagation.
  - Allowed deps: zaroxi-kernel-collections, zaroxi-kernel-async.
- zaroxi-core-commands
  - Purpose: typed command registry, command execution model, undo/redo hooks.
  - Allowed deps: kernel crates.
- zaroxi-core-runtime
  - Purpose: thin runtime glue for tasks and IO scheduling (not platform runtime).
  - Allowed deps: zaroxi-kernel-async, kernel crates.
- zaroxi-core-scheduler
  - Purpose: internal high-performance scheduler for short-lived UI/IO tasks.
  - Allowed deps: zaroxi-core-runtime, kernel.
- zaroxi-core-task
  - Purpose: task primitives used by core and application layers.
  - Allowed deps: zaroxi-core-runtime, kernel.
- zaroxi-core-threading
  - Purpose: cross-platform threading primitives and pools (only in core/infrastructure adapters).
  - Allowed deps: kernel.
- zaroxi-core-text
  - Purpose: public API for text buffers, edits, snapshots (zero-copy where possible).
  - Allowed deps: zaroxi-kernel-collections, zaroxi-core-text-rope.
- zaroxi-core-text-buffer
  - Purpose: high-level buffer model used by domain buffer; implements change history and lightweight snapshots.
  - Allowed deps: zaroxi-core-text, kernel.
- zaroxi-core-text-rope
  - Purpose: rope data structure optimized for huge files and zero-copy slices.
  - Allowed deps: kernel only.
- zaroxi-core-text-edit
  - Purpose: efficient edit application, ranges translation.
  - Allowed deps: core-text-rope.
- zaroxi-core-text-diff
  - Purpose: incremental diff algorithms for multi-megabyte files.
  - Allowed deps: kernel, core-text-rope.
- zaroxi-core-render
  - Purpose: render API abstraction (device-agnostic).
  - Allowed deps: kernel.
- zaroxi-core-render-backend
  - Purpose: backend adapters (wgpu/vulkan/metal) kept behind strict feature flags.
  - Allowed deps: core-render, kernel, backend crates gated by features.
- zaroxi-core-render-graph
  - Purpose: render graph system for resource & pass scheduling.
  - Allowed deps: core-render, kernel.
- zaroxi-core-render-pipeline
  - Purpose: shader pipeline & pipeline state management.
  - Allowed deps: core-render, render-backend.
- zaroxi-core-render-resource
  - Purpose: GPU resource management & lifetime tracking.
  - Allowed deps: core-render, render-backend.
- zaroxi-core-render-text
  - Purpose: glyph shaping, font atlas management, GPU text uploading.
  - Allowed deps: core-render, kernel.
- zaroxi-core-render-ui
  - Purpose: immediate/retained UI primitives, batching system.
  - Allowed deps: core-render, core-layout.
- zaroxi-core-render-scene
  - Purpose: scene graph for compositing editor overlays and complex visuals.
  - Allowed deps: core-render, core-layout.
- zaroxi-core-render-compositor
  - Purpose: final composition & post-processing.
  - Allowed deps: core-render, render-resource.
- zaroxi-core-layout
  - Purpose: layout primitives (flex-like), measurement.
  - Allowed deps: kernel.
- zaroxi-core-ui
  - Purpose: low-level widgets and input integration (not app-level widgets).
  - Allowed deps: core-layout, core-render, kernel.

--- Domain layer (pure logic, no IO)

- zaroxi-domain-editor
  - Purpose: editor use-cases (commands, ranges, decoration model).
  - Allowed deps: zaroxi-domain-buffer, kernel, core-text (as trait-only).
- zaroxi-domain-workspace
  - Purpose: workspace model and project metadata (no FS IO).
  - Allowed deps: kernel, domain-project.
- zaroxi-domain-project
  - Purpose: project model, project graph, build targets (pure model).
  - Allowed deps: kernel, domain-workspace.
- zaroxi-domain-buffer
  - Purpose: buffer abstraction; ties domain semantics to core-text buffer API (via traits).
  - Allowed deps: core-text (trait-only), kernel.
- zaroxi-domain-selection
  - Purpose: selection model, multiple selections / cursors.
  - Allowed deps: kernel.
- zaroxi-domain-cursor
  - Purpose: cursor movement semantics & visibilities.
  - Allowed deps: kernel, domain-selection.
- zaroxi-domain-history
  - Purpose: change history, checkpointing (pure logic).
  - Allowed deps: kernel.
- zaroxi-domain-ai
  - Purpose: AI context models and prompt builders (pure logic, no external IO).
  - Allowed deps: kernel, domain-buffer.
- zaroxi-domain-settings
  - Purpose: canonical settings model (no IO).
  - Allowed deps: kernel, kernel-config.

--- Application layer (orchestration, light IO via adapters)

- zaroxi-application-editor
  - Purpose: orchestrates editor features (commands → domain → core).
  - Allowed deps: domain-_, core-_, intelligence-\* (via interfaces), kernel.
- zaroxi-application-workspace
  - Purpose: orchestrates workspace open/close, indexing triggers, watchers (uses workspace-\* via adapters).
  - Allowed deps: domain-workspace, workspace-\*, kernel.
- zaroxi-application-project
  - Purpose: build & task orchestration (calls external infra via adapters).
  - Allowed deps: domain-project, infrastructure-\* (via trait adapters).
- zaroxi-application-buffer
  - Purpose: buffer lifecycle, syncing domain buffer with core text buffer.
  - Allowed deps: domain-buffer, core-text, workspace-\*.
- zaroxi-application-command
  - Purpose: command registry glue and permission checks.
  - Allowed deps: core-commands, domain-\*.
- zaroxi-application-search
  - Purpose: cross-file search orchestration, scalable indexing.
  - Allowed deps: workspace-index, domain-buffer, core-text.
- zaroxi-application-ai
  - Purpose: high-level AI features orchestration: copilots, assistants, agent orchestration interfaces.
  - Allowed deps: intelligence-\*, domain-ai, infrastructure-rpc (adapter).
- zaroxi-application-navigation
  - Purpose: navigation (go-to-definition, symbol indices).
  - Allowed deps: platform-\*, domain-project, core-text.
- zaroxi-application-refactor
  - Purpose: refactoring orchestration and safe-apply mechanisms.
  - Allowed deps: application-command, domain-buffer, workspace-patch.

--- Intelligence layer (AI subsystems)

- zaroxi-intelligence-agent
  - Purpose: agent runtime (planners, executors) — core agent models and control loop (no direct outbound network calls).
  - Allowed deps: kernel, intelligence-planning, intelligence-memory.
- zaroxi-intelligence-planning
  - Purpose: plan generation, step decomposition.
  - Allowed deps: kernel, intelligence-context.
- zaroxi-intelligence-memory
  - Purpose: vector DB / memory cache patterns (in-memory impls only).
  - Allowed deps: kernel, intelligence-embedding.
- zaroxi-intelligence-context
  - Purpose: context packing, prompt building algorithms.
  - Allowed deps: kernel, domain-ai.
- zaroxi-intelligence-tools
  - Purpose: tools/adapters the agent can call (filesystem, git) designed as trait-only interfaces here; implementations live in
    infrastructure-_ or platform-_.
  - Allowed deps: kernel, intelligence-agent.
- zaroxi-intelligence-orchestrator
  - Purpose: multi-agent coordination, scheduling, sandboxing orchestration.
  - Allowed deps: intelligence-agent, security-sandbox.
- zaroxi-intelligence-eval
  - Purpose: evaluation harnesses for agent outputs.
  - Allowed deps: intelligence-agent, kernel.
- zaroxi-intelligence-embedding
  - Purpose: embedding vector utilities and interfaces (no remote calls).
  - Allowed deps: kernel, intelligence-memory.

--- Platform layer (language & tooling adapters)

- zaroxi-platform-lsp
  - Purpose: LSP adapter & glue.
  - Allowed deps: application-_, domain-_, infrastructure-rpc.
- zaroxi-platform-syntax
  - Purpose: language grammars, parser registry (Tree-sitter adapters), grammar runtime loaders.
  - Allowed deps: kernel, core-text, feature-gated dynamic loaders; runtime parts live in infrastructure adapters.
- zaroxi-platform-debugger
  - Purpose: debug protocol adapters and session management.
  - Allowed deps: application-_, infrastructure-_.
- zaroxi-platform-terminal
  - Purpose: terminal emulator integration for app.
  - Allowed deps: core-input, infrastructure-rpc.
- zaroxi-platform-git
  - Purpose: git model + command adapters (no direct exec; trait-only).
  - Allowed deps: kernel, application-workspace, infrastructure-storage (adapter impl).
- zaroxi-platform-plugin
  - Purpose: plugin host API, sandbox contract, ABI; trait definitions only — implementations live in security-sandbox and infra
    adapters.
  - Allowed deps: kernel, security-\* (interfaces).

--- Workspace layer (filesystem & state)

- zaroxi-workspace-files
  - Purpose: pure model of files in workspace (no FS IO).
  - Allowed deps: kernel, domain-workspace.
- zaroxi-workspace-index
  - Purpose: indexing model and metadata (no IO).
  - Allowed deps: kernel, domain-project.
- zaroxi-workspace-history
  - Purpose: lightweight timeline/undo across workspace operations.
  - Allowed deps: kernel, domain-history.
- zaroxi-workspace-patch
  - Purpose: patch model & preview application (pure logic).
  - Allowed deps: kernel, domain-buffer.
- zaroxi-workspace-watcher
  - Purpose: trait-only watcher interface; platform-specific impls in infrastructure.
  - Allowed deps: kernel.
- zaroxi-workspace-cache
  - Purpose: caching policy interfaces and in-memory cache impl.
  - Allowed deps: kernel.

--- Collaboration layer (real-time)

- zaroxi-collaboration-sync
  - Purpose: session sync orchestration (uses CRDT ops).
  - Allowed deps: collaboration-crdt, infrastructure-rpc, kernel.
- zaroxi-collaboration-crdt
  - Purpose: CRDT implementations for text and presence (PLDT/RCU designs).
  - Allowed deps: kernel, core-text-rope.
- zaroxi-collaboration-presence
  - Purpose: presence model & ephemeral state.
  - Allowed deps: kernel.
- zaroxi-collaboration-session
  - Purpose: session lifecycle & permissions integration.
  - Allowed deps: collaboration-sync, security-auth.

--- Infrastructure (adapters only, implement traits)

- zaroxi-infrastructure-rpc
  - Purpose: JSON-RPC / protobuf transport implementation (server & client adapters).
  - Allowed deps: kernel, infrastructure-logging, infrastructure-tracing.
- zaroxi-infrastructure-http
  - Purpose: HTTP server adapters (optional), websocket transports.
  - Allowed deps: kernel, infrastructure-tracing.
- zaroxi-infrastructure-storage
  - Purpose: implementations for FS/remote storage (S3, gcs) behind trait.
  - Allowed deps: kernel, security-sandbox (for sandboxed IO).
- zaroxi-infrastructure-settings
  - Purpose: settings persistence and loaders (OS-specific impl).
  - Allowed deps: kernel.
- zaroxi-infrastructure-permissions
  - Purpose: permission evaluation engine & RBAC mapping.
  - Allowed deps: kernel, security-policy.
- zaroxi-infrastructure-logging
  - Purpose: logging sink adapters (OTel, file).
  - Allowed deps: kernel.
- zaroxi-infrastructure-metrics
  - Purpose: metrics exporters & collectors.
  - Allowed deps: kernel.
- zaroxi-infrastructure-tracing
  - Purpose: tracing adapters (OpenTelemetry).
  - Allowed deps: kernel.

--- Security layer

- zaroxi-security-sandbox
  - Purpose: process/container sandbox abstraction for running plugins/agents.
  - Allowed deps: kernel, infrastructure-storage (controlled), infrastructure-rpc (for proxying).
- zaroxi-security-policy
  - Purpose: permission language and policy evaluation.
  - Allowed deps: kernel.
- zaroxi-security-validation
  - Purpose: artifact validation & integrity checks on plugin packages.
  - Allowed deps: kernel.
- zaroxi-security-auth
  - Purpose: authentication flows & token validation (pluggable).
  - Allowed deps: kernel, infrastructure-settings.

--- Interface / Entry

- zaroxi-interface-app
  - Purpose: desktop/web app shell and orchestration (entrypoint).
  - Allowed deps: application-_, infrastructure-_, kernel.
- zaroxi-interface-editor
  - Purpose: the concrete editor UI combining core-ui, domain models, and application features.
  - Allowed deps: application-\*, core-ui, core-render.
- zaroxi-interface-gui
  - Purpose: GUI toolkit integration (native windowing & event loop).
  - Allowed deps: interface-app, core-render-backend (feature gated).
- zaroxi-interface-cli
  - Purpose: CLI entrypoints for headless operations.
  - Allowed deps: application-\*, infrastructure-rpc.

---

## 7. Governance & enforcement (how to keep architecture intact)

- Workspace-level Cargo.toml: explicit path entries for each crate; no wildcard globs.
- tools/crate-lint: a Rust binary that loads cargo metadata and validates:
  - dependency rules (forbidden edges),
  - rename-map enforcement,
  - no-public re-exports from infra into domain/core direct imports.
- CI: run cargo build with workspace features, run tools/crate-lint, run cargo deny for license & dependency checks.
- Tests: for each layer add "layer-internal" unit tests and "layer-boundary" integration tests to assert trait-only interfaces.
- Code review: PR template requires "layer-impact" checklist.

---

## 8. Performance & security design decisions (concrete)

- Text engine: zero-copy ropes with fragment sharing; snapshots are Arc-backed rope roots; diff uses rope-deltas and range maps.
- Concurrency: actor-ish scheduler for UI; lock-free ring buffers for input/events; read-optimized immutable data with atomic
  reference counting.
- GPU: render-graph schedules passes; batched UI drawing; glyph atlases uploaded via streaming buffers; partial re-render via damage
  tracking.
- AI sandboxing: agents execute in zaroxi-security-sandbox processes with restricted filesystem & network; all plugin / agent IO goes
  through infrastructure adapters subject to policy checks.
- Plugin model: two-tier model:
  - Native plugin (Rust) using plugin ABI + sandbox runner,
  - Remote plugin (process over RPC) using protocol defined in platform-plugin crate.
- Observability: each crate must emit structured events via infrastructure-tracing and infrastructure-metrics adapters. Core crates
  expose metrics hooks.

---

## 9. Migration plan (high level, prioritized)

Phase 0 — Prepare:

- Add this architecture file to repo.
- Add tools/crate-lint and workspace Cargo manifest (minimal).
  Phase 1 — Kernel + Core skeletons:
- Create kernel-\* crates and minimal exports.
- Create core-text-rope and core-text stub.
  Phase 2 — Domain split:
- Move pure logic into domain-\* crates.
  Phase 3 — Application & Intelligence adapters:
- Create application-_ & intelligence-_ stubs and trait definitions.
  Phase 4 — Platform & Infrastructure:
- Move infra adapter implementations and platform pieces.
  Phase 5 — Security & Collaboration:
- Implement sandbox, CRDT engine, and instrument telemetry.
  Phase 6 — Validation:
- Run workspace CI, fix dependency violations, adopt cargo-deny.

Each phase: create minimal crates with public interfaces + tests; iterate to full implementation.

---

## 10. Example enforcement rules (to codify in tools/crate-lint)

- If crate A depends on crate B, ensure layer(A) >= layer(B) according to allowed flows.
- No domain crate may depend on any infrastructure crate.
- All infra crates must not be depended upon by core or domain.
- All runtime unsafe/use of dynamic loading must be feature gated and audited.

---

## 11. Appendices

Appendix A: Crate visibility & API rules

- Interior crates (core-_, kernel-_) should limit public APIs to minimal surface area. Prefer sealed traits and explicit facade
  crates (e.g., core-text is a facade, core-text-rope is internal).
- Use Rust features to gate backend-specific impls (e.g., "wgpu-backend").

Appendix B: Example Cargo workspace changes (illustrative)

- Add explicit [workspace] members for every listed crate and top-level tools.
- Add a crate-lint check in CI to prevent accidental dependency-rule regressions.

---

## 12. Next actions for me (if you want me to edit the repo)

If you want me to start implementing this in the repo, tell me which of the following files/directories you will add to the chat (I
will need the actual files to edit):

- workspace/Cargo.toml
- existing crates that should be renamed (I will provide exact SEARCH/REPLACE blocks to rename path entries)
- any crate root Cargo.toml that requires modification to change names or dependencies

I will wait for you to add those files to the chat. Do not change anything yet — I will only modify files you add.

---
