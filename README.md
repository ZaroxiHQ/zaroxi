# Zaroxi Studio: AI-First IDE

⚠️ Heavily Under Development: Zaroxi Studio is currently in active development. APIs, features, and architecture are subject to change. We welcome early adopters and contributors to help shape the project.

[![CI](https://github.com/mujaxso/zaroxi/actions/workflows/ci.yml/badge.svg)](https://github.com/mujaxso/zaroxi/actions/workflows/ci.yml) [![Security Audit](https://github.com/mujaxso/zaroxi/actions/workflows/security-audit.yml/badge.svg)](https://github.com/mujaxso/zaroxi/actions/workflows/security-audit.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

Overview
- Zaroxi is a Rust-native, crate-first editor and IDE architecture. The repository contains core libraries, engine subsystems, infrastructure adapters, intelligence scaffolding, and the desktop harness used to exercise editor flows prior to Phase 9.
- Status: Active development — documentation & architecture modernization (pre‑Phase‑9).

Key current capabilities
- Crate-based architecture with explicit layer boundaries and naming conventions.
- Working editor primitives (buffers, transactions, view/projection models) and engine subsystems.
- Desktop harness (`zaroxi-interface-desktop` / `zaroxi-interface-app`) that exercises workspace flows, buffer switching, checkpoint restore, pending-close/session-close handling, command bar, transcript/shell composition, AI explanation/projection, and GPU shell rendering.
- Inline AI explanation/projection (editor-side explain-only flows).
- Desktop/app harness tests are passing and used as the primary runtime verification path.

What is not implemented yet (important limits)
- Real disk-backed workspace/file persistence (planned for Phase 9).
- Production LSP integration (Phase 10 target).
- Full AI edit/apply flow (AI projection/explain exists; safe apply is planned for Phase 11).
- Some infrastructure adapters and platform integrations remain scaffolding and will mature in upcoming phases.

Key Features (detailed)
- Modular Layers: Kernel → Core → Domain → Application → Interface. Special namespaces for `infrastructure`, `intelligence`, and `security`.
- Editor Core: In-memory buffer primitives, transactions, checkpoints, and view/projection models suitable for building an editor shell.
- GPU Shell: Engine rendering primitives and backend adapters used by the desktop harness to exercise rendering paths.
- Desktop Harness: A runnable harness that composes interface → application → domain → core to exercise open/close, buffer operations, command flows, and AI projection.
- AI Explanation: Editor-side AI explain/projection for previewing suggestions and explanations; end-to-end AI apply remains planned.

Documentation
Comprehensive documentation is available in the `docs/` directory:
- `docs/architecture.md` — High-level system design and layer responsibilities
- `docs/crates.md` — Detailed crate documentation and recommended reading order
- `docs/rpc.md` — RPC scaffold and communication role
- `docs/security.md` — Security crate family and posture
- `docs/roadmap.md` — Development roadmap and next phases

Contact & Links
- Website: https://www.zaroxi.com
- GitHub: https://github.com/mujaxso/zaroxi
- Documentation: https://docs.zaroxi.com
- Twitter: @zaroxi_studio
- Email: contact@zaroxi.com

Build & Run
Requirements: Rust (stable, recent; 1.75+ recommended)

Build workspace
```bash
cargo build --workspace
```

Run desktop harness
```bash
# Build and run primary desktop harness
cargo run -p zaroxi-interface-desktop --release

# Development/debug
cargo run -p zaroxi-interface-desktop
```

Run tests
```bash
# All workspace tests
cargo test --workspace

# Desktop harness tests
cargo test -p zaroxi-interface-desktop
```

Roadmap (short)
- Current (pre-Phase-9): Rust-native editor shell, desktop harness, engine primitives, AI explain/projection, tests.
- Phase 9: Disk-backed workspace persistence and file-backed buffers.
- Phase 10: LSP baseline and diagnostics plumbing.
- Phase 11: Safe AI edit/apply with preview & verification.
- Phase 12: Performance and incremental rendering improvements.
- Phase 13: Productization (packaging, installers, release infra).
- Phase 14: Alpha release.

Contribution & Documentation note
- Keep `docs/` in sync with the crate layout and architecture changes. When adding, renaming, or removing crates, update `docs/crates.md` and `docs/architecture.md`.
- Use the roadmap to describe planned work; avoid describing planned features as implemented.

License
- MIT — See `LICENSE` for details.
