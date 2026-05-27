# Zaroxi

Zaroxi is a Rust-native, crate-based editor/IDE architecture and working editor harness. This repository contains the core libraries, runtime engines, and the desktop harness used to exercise editor features and flows prior to Phase 9.

Status
- Stage: Active development — documentation & architecture modernization (pre-Phase-9).
- Current state: A working Rust-native editor shell and desktop harness built from the `interface-*` and `application-*` crates. Desktop tests and harness integrations are passing. Core editor and engine primitives exist and are actively evolving.

What Zaroxi is
- A modular, crate-oriented editor and IDE architecture implemented in Rust.
- Focused on a rigorous layer model (kernel → core → domain → application → interface) and explicit infrastructure/intelligence/security namespaces.
- Provides a working desktop harness (via `zaroxi-interface-app` / `zaroxi-interface-desktop`) that exercises workspace open, buffer open, editor state, refresh, active buffer switching, AI explanation/projection, checkpoint restore, session/close flows, command bar, transcript/shell composition, and GPU shell integration.

What is implemented now (key capabilities)
- A crate-based architecture with clear layer boundaries and naming conventions.
- Working editor core primitives (buffers, transactions, view/projection models) and many core engine subsystems.
- Desktop harness that exercises editor flows and the GPU shell using the `interface` and `application` stacks.
- Inline AI explanation/projection features (editor-side projections and explain-only flows).
- Tests and harnesses for the desktop/app path are passing.

What is not implemented yet (important limits)
- Real disk-backed workspace/file persistence (planned for Phase 9).
- Full LSP (Language Server Protocol) integration (Phase 10 target).
- Production AI edit/apply flow (AI explain/projection exists; applying edits via AI is incomplete — Phase 11 target).
- Some integrations and platform adapters remain scaffolding (storage, full remote persistence, and some infra adapters).

Architecture Overview (concise)
- Kernel: `zaroxi-kernel-*` — minimal, stable primitives (IDs, types, small helpers).
- Core: `zaroxi-core-*` — editor engine, rendering, input, scheduling, workspace helpers.
- Domain: `zaroxi-domain-*` — workspace, buffer semantics, session, plugin domain models.
- Application: `zaroxi-application-*` — feature orchestration (editor composition, workspace flows, command routing).
- Interface: `zaroxi-interface-*` — concrete entry points (desktop harness, CLI, theming).
- Special namespaces: `zaroxi-intelligence-*`, `zaroxi-infrastructure-*`, `zaroxi-security-*` for AI logic, adapters, and security primitives.

Repository layout (high-level)
- `crates/` — workspace crates (kernel, core, domain, application, interface, intelligence, infrastructure, security).
- `docs/` — architecture, crates guide, RPC, security, roadmap and related docs.
- `Cargo.toml` — workspace configuration.

Main crates to know first
- `zaroxi-interface-desktop` / `zaroxi-interface-app` — desktop harness and app shell.
- `zaroxi-application-workspace` — workspace orchestration (open/close, indexing triggers, workspace flows).
- `zaroxi-core-editor-buffer` and `zaroxi-domain-buffer` — buffer implementations and higher-level buffer semantics.
- `zaroxi-core-engine-*` (render, view, input) — rendering and engine primitives; GPU shell integrations.
- `zaroxi-infrastructure-rpc` and `zaroxi-application-remote` — remote scaffolding and orchestration.
- `zaroxi-intelligence-*` — agent, context, planning crates (AI scaffolding and tools).
- `zaroxi-security-*` — audit, auth, policy, validation, sandbox primitives.

Build
- Requires Rust toolchain (stable recent, e.g., 1.75+ recommended).

Build workspace
```bash
# From repository root
cargo build --workspace
```

Run desktop harness
- The current desktop harness is driven by the `interface` crates. From the repository root you can run the desktop harness binary:
```bash
# Build and run the desktop harness
cargo run -p zaroxi-interface-desktop --release
# or for development/debug builds
cargo run -p zaroxi-interface-desktop
```
Notes:
- The crate `zaroxi-interface-desktop` is the primary local harness used during development. If the binary name differs, run `cargo run -p zaroxi-interface-desktop -- --help` or inspect `crates/zaroxi-interface-desktop/Cargo.toml` for the exact binary target.

Run desktop/app tests
```bash
# Run all tests in the workspace
cargo test --workspace

# Run tests for the desktop harness specifically
cargo test -p zaroxi-interface-desktop
```

Where to read more
- See `docs/architecture.md` for the layer model and runtime paths.
- See `docs/crates.md` for a guided crate inventory and reading order.
- See `docs/roadmap.md` for the phases starting from the current pre-Phase-9 state.
- See `docs/rpc.md` for the current RPC scaffold and role.
- See `docs/security.md` for the security crate family and current state.

Short roadmap summary (from current state)
- Current (pre-Phase-9): Rust-native editor shell, desktop harness, engine primitives, AI explanation/projection, tests.
- Phase 9: Real disk-backed workspace and file persistence (primary next milestone).
- Phase 10: LSP baseline and richer editor language integration.
- Phase 11: AI edit/apply flow (safe apply, verification, preview, user confirmation).
- Phase 12: Performance engineering and incremental rendering.
- Phase 13: Productization and polishing (installer, updates, platform packaging).
- Phase 14: Alpha release.

Contribution & documentation note
- This repository is undergoing a documentation modernization pass. Keep docs accurate: when you add, rename, or remove crates, update `docs/crates.md` and `docs/architecture.md` accordingly.
- Do not describe planned features as implemented. Use the roadmap to describe near-term plans and keep the README focused on the current, verifiable state.

License
- Zaroxi is open source. See `LICENSE` for details.

Contact
- See project `CONTRIBUTING.md` and `docs/` for contribution guidance and further details.
