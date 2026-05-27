# Zaroxi

Zaroxi is a Rust-native, crate-first editor architecture and working editor harness. This repository contains the core libraries, engine subsystems, infrastructure adapters, and the desktop harness used to exercise editor flows prior to Phase 9.

Summary
- Status: Active development — documentation and architecture modernization (pre‑Phase‑9).
- Current state: A working Rust-native editor shell and desktop harness (implemented via `zaroxi-interface-*` and `zaroxi-application-*` crates). Desktop harness tests are passing; core editor and engine primitives are under active development.

What Zaroxi is
- A modular, crate-oriented editor and IDE architecture implemented in Rust, designed for clear layer separation and testability.
- Focused on a rigorous layer model (kernel → core → domain → application → interface) and dedicated namespaces for infrastructure, intelligence, and security.

Key current capabilities
- Crate-based architecture with explicit layer boundaries and naming conventions.
- Working editor primitives (buffers, transactions, view/projection models) and engine subsystems.
- Desktop harness that exercises workspace flows, buffer switching, checkpoint restore, pending-close/session-close handling, command bar, transcript/shell composition, AI explanation/projection, and GPU shell rendering.
- Inline AI explanation/projection (editor-side projections used for explain-only flows).

Important limitations (what is not implemented yet)
- Real disk-backed workspace/file persistence (planned for Phase 9).
- Production LSP integration (Phase 10 target).
- Full AI edit/apply flow (AI projection/explain exists; safe apply is planned for Phase 11).
- Some infrastructure adapters and platform integrations are scaffolding and will mature in upcoming phases.

Architecture (concise)
- Kernel: `zaroxi-kernel-*` — tiny stable primitives and canonical types.
- Core: `zaroxi-core-*` — engine, editor primitives, rendering, input, scheduling.
- Domain: `zaroxi-domain-*` — workspace, buffer semantics, session, settings, plugins.
- Application: `zaroxi-application-*` — feature orchestration and composition.
- Interface: `zaroxi-interface-*` — concrete entrypoints (desktop harness, CLI, theme assets).
- Special namespaces: `zaroxi-infrastructure-*`, `zaroxi-intelligence-*`, `zaroxi-security-*`.

Repository layout (high-level)
- `crates/` — workspace crates: kernel, core, domain, application, interface, infrastructure, intelligence, security.
- `docs/` — architecture, crates guide, RPC, security, roadmap, and related documentation.
- `Cargo.toml` — workspace configuration.

Core crates to inspect first
- `zaroxi-interface-desktop`, `zaroxi-interface-app` — desktop harness and app shell.
- `zaroxi-application-workspace` — workspace lifecycle and orchestration.
- `zaroxi-core-editor-buffer`, `zaroxi-domain-buffer` — buffer primitives and higher-level semantics.
- `zaroxi-core-engine-render`, `zaroxi-core-engine-view` — rendering and view composition primitives.
- `zaroxi-infrastructure-rpc`, `zaroxi-application-remote` — RPC scaffolding and remote orchestration.
- `zaroxi-intelligence-agent`, `zaroxi-intelligence-context` — AI agent scaffolding and context packing.

Build
- Requires a recent Rust toolchain (stable, e.g., 1.75+ recommended).

Build workspace

```bash
# From repository root
cargo build --workspace
```

Run the desktop harness

```bash
# Build and run the primary desktop harness
cargo run -p zaroxi-interface-desktop --release

# Development/debug build
cargo run -p zaroxi-interface-desktop
```

Notes: the harness is provided by `zaroxi-interface-desktop`. Inspect `crates/zaroxi-interface-desktop/Cargo.toml` for the exact binary target if necessary.

Run tests

```bash
# All workspace tests
cargo test --workspace

# Desktop harness tests only
cargo test -p zaroxi-interface-desktop
```

Where to read more
- `docs/architecture.md` — canonical architecture and layer responsibilities.
- `docs/crates.md` — guided crate inventory and reading order.
- `docs/roadmap.md` — concrete next phases from the current state through Phase 14.
- `docs/rpc.md` — role of RPC and current scaffold.
- `docs/security.md` — security crate family and conservative posture.

Roadmap (short)
- Current: Rust-native editor shell, desktop harness, engine primitives, AI explanation/projection, tests.
- Phase 9: Disk-backed workspace persistence and file-backed buffers.
- Phase 10: LSP baseline and diagnostics plumbing.
- Phase 11: Safe AI edit/apply with preview & verification.
- Phase 12: Performance and incremental rendering improvements.
- Phase 13: Productization (packaging, installers, release infra).
- Phase 14: Alpha release.

Contribution and documentation note
- Keep docs in `docs/` in sync with crate changes. When adding, renaming, or removing crates, update `docs/crates.md` and `docs/architecture.md`.
- Do not describe planned features as implemented; use the roadmap for planned work and the README for the current, verifiable state.

License
- See `LICENSE` for license details.
