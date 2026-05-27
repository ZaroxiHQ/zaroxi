# Zaroxi Studio: AI-First IDE

⚠️ Heavily Under Development: Zaroxi Studio is currently in active development. APIs, features, and architecture are subject to change. We welcome early adopters and contributors to help shape the project.

[![CI](https://github.com/mujaxso/zaroxi/actions/workflows/ci.yml/badge.svg)](https://github.com/mujaxso/zaroxi/actions/workflows/ci.yml) [![Security Audit](https://github.com/mujaxso/zaroxi/actions/workflows/security-audit.yml/badge.svg)](https://github.com/mujaxso/zaroxi/actions/workflows/security-audit.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

Overview

Zaroxi Studio is an open-source, AI-first integrated development environment built in Rust. It combines modern IDE features with AI-powered assistance to create a next-generation development experience.

This repository contains the crate-based architecture, core libraries, engine subsystems, infrastructure adapters, intelligence scaffolding, and the desktop harness used to exercise editor flows prior to Phase 9.

Quick status
- Stage: Active development — documentation & architecture modernization (pre‑Phase‑9).
- Current: A working Rust-native editor shell and desktop harness (implemented via `zaroxi-interface-*` and `zaroxi-application-*` crates). Desktop harness tests are passing; core editor and engine primitives are under active development.

Key current capabilities
- Modular crate-first architecture with explicit layer boundaries.
- Editor primitives: in-memory buffers, transactions, checkpoints, and view/projection models.
- Desktop harness: exercises workspace open/close, buffer switching, checkpoint restore, pending-close/session-close flows, command bar, transcript/shell composition, AI explanation/projection, and GPU shell rendering.
- AI explanation/projection: editor-side explain-only flows for previewing suggestions.

Important limitations
- Real disk-backed workspace/file persistence is planned for Phase 9.
- Production LSP integration is planned for Phase 10.
- Full AI edit/apply flow (safe apply) is planned for Phase 11.

Key Features (concise)
- Clear layer model: Kernel → Core → Domain → Application → Interface, with separate namespaces for `infrastructure`, `intelligence`, and `security`.
- Engine-first design: rendering, input, scheduling, and GPU shell primitives are core to the architecture.
- Testable harness: the desktop harness is the primary verification path for runtime behaviors.

Documentation
Comprehensive documentation is available in the `docs/` directory:
- `docs/architecture.md` — High-level system design and layer responsibilities
- `docs/crates.md` — Crate inventory and recommended reading order
- `docs/rpc.md` — RPC scaffold and role
- `docs/security.md` — Security crate family and posture
- `docs/roadmap.md` — Development roadmap and next phases

Build & run
Prerequisite: Rust (stable, recent; 1.75+ recommended)

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
- Current: Rust-native editor shell, desktop harness, engine primitives, AI explanation/projection, tests.
- Phase 9: Disk-backed workspace persistence and file-backed buffers.
- Phase 10: LSP baseline and diagnostics plumbing.
- Phase 11: Safe AI edit/apply with preview & verification.
- Phase 12: Performance and incremental rendering improvements.
- Phase 13: Productization (packaging, installers, release infra).
- Phase 14: Alpha release.

Contributing & support
- If you find Zaroxi useful, please consider starring the project on GitHub — it helps improve visibility, attract contributors, and signals community interest.
- Contributions welcome: open issues, propose designs, and submit pull requests. Keep documentation updated (`docs/architecture.md`, `docs/crates.md`) when crates change.

Contact & links
- Website: https://www.zaroxi.com
- GitHub: https://github.com/mujaxso/zaroxi
- Documentation: https://docs.zaroxi.com
- Twitter: @zaroxi_studio
- Email: contact@zaroxi.com

License
- MIT — See `LICENSE` for details.
