# Zaroxi Studio — AI-first editor (Rust native)

One-line value proposition: A Rust-native, crate-based AI-first editor/IDE focused on safe, testable editor primitives and an extensible intelligence layer.

---

## What Zaroxi is

Zaroxi is a crate-first editor and IDE runtime written in Rust. It is organized as small, responsibility-focused crates arranged into strict layers (kernel → core → domain → application → interface) with separate families for infrastructure, intelligence, and security. The repository hosts the working desktop harness used to exercise runtime behaviors prior to Phase 9.

---

## Current state of the project

| Area | Status |
|---|---:|
| Desktop harness (runtime flows) | Working — exercised by `zaroxi-interface-desktop` and `zaroxi-interface-app` |
| Editor engine primitives | Implemented (in-memory buffers, transactions, view/projection) |
| Disk-backed persistence | Planned (Phase 9) |
| Production LSP integration | Planned (Phase 10) |
| AI edit/apply pipeline | Planned (Phase 11) |


---

## What works today

- Modular crate-first architecture and clear layer model
- Desktop harness: open/close flows, pending-close/session behavior, command bar flow, and GPU shell rendering
- Engine primitives: in-memory buffers, transactions, checkpoints, view/projection models
- Editor-side AI explanation/projection (preview-only flows)
- Tests covering desktop harness and core primitives

---

## What is not done yet

- Real disk-backed workspace/file persistence (Phase 9)
- Full LSP client/server baseline for diagnostics (Phase 10)
- Safe end-to-end AI edit/apply with preview + verification (Phase 11)
- Some infrastructure adapters and hardened transports for remote deployment

---

## Architecture at a glance

- Kernel: `zaroxi-kernel-*` — minimal types and traits
- Core: `zaroxi-core-*` — engine/runtime (buffers, rendering, input)
- Domain: `zaroxi-domain-*` — business logic and models (workspace, buffer semantics)
- Application: `zaroxi-application-*` — feature composition (workspace orchestration)
- Interface: `zaroxi-interface-*` — concrete shells (desktop harness)

Special families: `zaroxi-infrastructure-*`, `zaroxi-intelligence-*`, `zaroxi-security-*`

---

## Repository layout at a glance

- `crates/` — workspace crates, grouped by family
- `apps/` — runtime harnesses and daemons (desktop harness, ai-daemon, workspace-daemon)
- `docs/` — project documentation (this is the canonical docs directory)
- `extensions/` — built-in language/extensions examples

---

## Key crates to know first

- `zaroxi-interface-desktop` / `zaroxi-interface-app` — start here to run the harness
- `zaroxi-application-workspace` — workspace lifecycle orchestration
- `zaroxi-core-editor-buffer` — in-memory buffer and transactions
- `zaroxi-core-engine-render*` — rendering and GPU shell primitives
- `zaroxi-intelligence-agent` — agent runtime scaffolding
- `zaroxi-infrastructure-rpc` / `zaroxi-application-remote` — RPC and remote orchestration

---

## Build instructions

Prerequisite: Rust stable (recent; 1.75+ recommended)

Build the entire workspace:

```bash
cargo build --workspace
```

Clean build cache:

```bash
cargo clean
```

---

## Run instructions

Run the primary desktop harness (development):

```bash
# Development (debug)
cargo run -p zaroxi-interface-desktop

# Release build
cargo run -p zaroxi-interface-desktop --release
```

Run supporting daemons (examples):

```bash
# AI daemon (local)
cargo run -p ai-daemon
```

---

## Test instructions

Run all workspace tests:

```bash
cargo test --workspace
```

Run tests for a single crate:

```bash
cargo test -p zaroxi-interface-desktop
```

CI-quality checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Docs map

See `docs/` for focused, file-by-file documentation:

- `docs/architecture.md` — system architecture and runtime flow
- `docs/crates.md` — crate family guide and recommended reading order
- `docs/rpc.md` — current RPC scaffold and status
- `docs/security.md` — security model and crate responsibilities
- `docs/roadmap.md` — near-term phase roadmap and checkpoints
- `docs/MISSING_FILES.md` — precise documentation and architecture gaps

---

## Near-term roadmap (concise)

- Phase 9: Disk-backed workspace persistence (storage adapters, crash-safe persistence)
- Phase 10: LSP baseline and diagnostics plumbing
- Phase 11: Safe AI edit/apply flow with preview & verification

---

## Start here (for new contributors)

- Clone the repo and run `cargo test -p zaroxi-interface-desktop` to exercise the harness tests
- Read `docs/crates.md` to find the right crate to work on
- Open a small PR that updates one crate's README or adds tests; refer to `docs/MISSING_FILES.md` for high-value tasks

---

## Contribution & documentation maintenance note

Keep documentation file-scoped and single-purpose. When you add or rename a crate, update `docs/crates.md` and `docs/architecture.md` as appropriate. Use the `docs/` link-check CI workflow to validate links.

---

License: MIT
