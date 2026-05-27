# Zaroxi Architecture

This document describes the current, canonical architecture for Zaroxi as it exists before Phase 9. It focuses on the active crate-based layer model, the primary runtime paths, and how the desktop harness fits into the system.

Layer model (top → bottom)
- Interface (`zaroxi-interface-*`): Entry points and concrete products (desktop harness, CLI, theming). Integrates application features into runnable artifacts.
- Application (`zaroxi-application-*`): Feature composition and orchestration built from domain and core primitives (workspace orchestration, command routing, editor composition).
- Domain (`zaroxi-domain-*`): Pure business logic and domain models (workspace, buffer semantics, sessions, settings, plugins).
- Core (`zaroxi-core-*`): Foundational engines and primitives used across the system (editor buffers, rendering primitives, input, scheduling, workspace helpers).
- Kernel (`zaroxi-kernel-*`): Minimal, stable primitives (IDs, small types, lightweight utilities).

Special namespaces
- Infrastructure (`zaroxi-infrastructure-*`): Adapters and implementations (RPC, storage adapters, networking, tracing, settings). Infrastructure crates implement trait contracts defined in core/domain.
- Intelligence (`zaroxi-intelligence-*`): Agent runtimes, planning, context packing, and evaluation utilities. Intelligence crates are intentionally isolated from interface/application layers and prefer pure data and algorithms.
- Security (`zaroxi-security-*`): Audit, auth, policy, sandbox and validation primitives.

Principles
- Clear layer boundaries: lower layers must not depend on higher layers.
- Small stable kernels: keep kernel crates minimal and dependency-light.
- Adapter pattern: platform-specific implementations live in infrastructure; core/domain define traits/contracts.
- Pragmatic modularity: crates are split along responsibility lines and named consistently: `zaroxi-{layer}-{...}`.

Layers and responsibilities (summary)
- Kernel (`zaroxi-kernel-*`)
  - Purpose: Extremely small, stable primitives and canonical types (IDs, small numeric helpers, protocol types, traits).
  - Allowed deps: Rust standard library and a minimal set of vetted externals. Kernel crates should not depend on other `zaroxi-*` crates.

- Core (`zaroxi-core-*`)
  - Purpose: Engine and runtime primitives used across the stack (rendering, input, scheduling, editor primitives, workspace helpers).
  - Allowed deps: Kernel crates and other core crates only.

- Domain (`zaroxi-domain-*`)
  - Purpose: Pure business logic and domain models (workspace model, buffer semantics, session lifecycle, settings schema, plugin contracts).
  - Allowed deps: Kernel and core crates and other domain crates.

- Application (`zaroxi-application-*`)
  - Purpose: High-level feature composition and orchestration: editor composition, workspace flows, command routing, search, navigation.
  - Allowed deps: Kernel, core, domain, and other application crates.

- Interface (`zaroxi-interface-*`)
  - Purpose: Concrete entry points and product shells (desktop harness, CLI, theming assets). Integrates application features into runnable artifacts.
  - Allowed deps: Kernel, core, domain, application, and other interface crates.

- Infrastructure (`zaroxi-infrastructure-*`)
  - Purpose: Adapter implementations for networking, storage, RPC, tracing, settings, process management, and platform-specific integrations.
  - Allowed deps: Kernel and core crates (implementations should depend on trait contracts from core/domain as appropriate).

- Intelligence (`zaroxi-intelligence-*`)
  - Purpose: Agent runtimes, context packing, planning, embeddings, evaluation tools and safe orchestration primitives for AI features.
  - Allowed deps: Kernel, core, domain (avoid depending on application or interface layers).

- Security (`zaroxi-security-*`)
  - Purpose: Policy language and evaluation, authentication primitives, audit models, sandbox helpers, and validation primitives.
  - Allowed deps: Kernel, core, domain (avoid depending on application/interface/infrastructure where possible).

Primary runtime paths
1. Desktop harness (interface-desktop / interface-app)
   - The desktop harness is the active developer-facing entry point. It composes `interface` → `application` → `domain` → `core` layers to exercise editor features and flows.
   - Current harness responsibilities: workspace open, buffer open, editor state refresh, active buffer switching, checkpoint restore, pending-close and session-close flows, command bar, transcript/shell composition, GPU shell rendering, and AI explanation/projection flows.
   - The harness is implemented via `zaroxi-interface-app` and `zaroxi-interface-desktop` crates; these are the starting point for running and testing desktop behaviors.

2. Application-workspace flow
   - `zaroxi-application-workspace` orchestrates workspace lifecycle events: open, close, indexing triggers, and interactions with application-level features (search, navigation, project metadata).
   - Workspace orchestration uses domain models in `zaroxi-domain-workspace` and `zaroxi-domain-buffer` and leverages core helpers for snapshots and patch application.

3. Editor buffer and view
   - Core buffer primitives: `zaroxi-core-editor-buffer` (in-memory rope-like representation, transactions, checkpoints).
   - Buffer semantics and higher-level behaviors: `zaroxi-domain-buffer` and editor composition in `zaroxi-application-editor`.
   - Rendering and view: `zaroxi-core-engine-*` crates provide view composition, render resource management, and GPU shell plumbing used by the desktop harness.

4. Intelligence integration (current)
   - Intelligence crates provide planning, context packing, and tooling for agent workflows.
   - Current AI functionality is primarily explanation/projection (editor-side AI explain flows). The end-to-end AI edit/apply path remains work-in-progress.

5. Infrastructure adapters
   - `zaroxi-infrastructure-rpc` provides RPC transport adapters. `zaroxi-application-remote` consumes RPC adapters to enable remote orchestration.
   - Storage adapters (local FS and other backends) are present as infrastructure crates but full disk-backed workspace persistence is targeted for Phase 9.

GPU shell and composition
- The GPU shell is part of the core engine rendering stack (`zaroxi-core-engine-render*` family) and is exercised by the desktop harness.
- The GPU shell provides device-agnostic render APIs and resource abstractions; backend-specific adapters (wgpu/Metal/etc.) are isolated behind feature flags and live in render-backend crates.

Current state before Phase 9 (concise)
- The project is a Rust-native, crate-first architecture with a functioning desktop harness that exercises editor and engine flows.
- Desktop harness and selected tests are passing and are the primary verification path for runtime behaviors.
- Real disk-backed persistence, a production LSP integration, and the AI edit/apply flow are intentionally not yet implemented.
- Infrastructure and adapter layers contain scaffolding and some implemented adapters, but several platform integrations are planned for Phase 9 and later.

Near-term planned evolution (high level)
- Phase 9: Implement disk-backed workspace persistence and file-backed buffers (storage adapters, crash-safe persistence).
- Phase 10: Establish an LSP baseline and tighten language integration (lsp client/server scaffolding, diagnostics plumbing).
- Phase 11: Produce a safe AI edit/apply flow with preview, verification, and human approval controls.
- Phase 12+: Performance and rendering improvements, packaging, and alpha/beta releases.

Historical notes
- Older references to prior UI experiments have been removed from the canonical architecture documentation. Any historical references that remain in commit history are strictly historical and should not be treated as the current product architecture.

Where to look next
- `docs/crates.md` — guided reading order for crates and advice for new contributors.
- `docs/rpc.md` — current RPC scaffold and role in the system.
- `docs/roadmap.md` — concrete phase plan from current state to Phase 14.
