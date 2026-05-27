# Crates Guide — Reading Order and Responsibilities

This guide groups the workspace crates by responsibility and recommends a sensible reading order for new contributors. It maps crate names to their roles and highlights the most important crates to understand first.

How crates are organized
- Naming: `zaroxi-{layer}-{name}` where `{layer}` is one of `kernel`, `core`, `domain`, `application`, `interface`, `infrastructure`, `intelligence`, or `security`.
- Layers enforce dependency direction: kernel → core → domain → application → interface. Special namespaces (infrastructure, intelligence, security) may depend on kernel/core/domain but avoid depending on application/interface.

Recommended reading order (short path)
1. Kernel: `zaroxi-kernel-types`, `zaroxi-kernel-core` — small stable primitives and canonical types.
2. Core editor primitives: `zaroxi-core-editor-buffer`, `zaroxi-core-commands`, `zaroxi-core-editor-model` — understand buffer storage, commands and editor model.
3. Application workspace: `zaroxi-application-workspace`, `zaroxi-application-editor`, `zaroxi-application-command` — how workspace lifecycle and editor features are composed.
4. Interface harness: `zaroxi-interface-app`, `zaroxi-interface-desktop` — the desktop harness entrypoint used to run and exercise flows.
5. Intelligence and infra: `zaroxi-intelligence-agent`, `zaroxi-infrastructure-rpc`, `zaroxi-infrastructure-storage` — scaffolding for agent workflows and adapters.

Key crates to know first
- `zaroxi-interface-desktop` / `zaroxi-interface-app` — desktop harness and app-level shell examined to run the editor harness locally.
- `zaroxi-application-workspace` — orchestrates open/close, workspace flows, and is a central integration point for many application features.
- `zaroxi-core-editor-buffer` — the in-memory buffer implementation used across the editor; understanding this crate is critical for editor behavior.
- `zaroxi-domain-buffer` — higher-level buffer semantics and behaviors built on top of core buffer primitives.
- `zaroxi-core-engine-render` / `zaroxi-core-engine-render-backend` — rendering primitives and backend adapters used by the GPU shell.
- `zaroxi-infrastructure-rpc` and `zaroxi-application-remote` — remote scaffolding and transport adapters (see `docs/rpc.md`).
- `zaroxi-intelligence-agent` / `zaroxi-intelligence-context` — agent runtime and context packing for AI flows.

Groups by responsibility (selective)
- Kernel: `zaroxi-kernel-core`, `zaroxi-kernel-types`, `zaroxi-kernel-errors`, `zaroxi-kernel-async`, `zaroxi-kernel-time`, `zaroxi-kernel-traits`.

- Core (editor & engine): `zaroxi-core-commands`, `zaroxi-core-editor-buffer`, `zaroxi-core-editor-model`, `zaroxi-core-editor-view`, `zaroxi-core-engine-render`, `zaroxi-core-engine-runtime`, `zaroxi-core-engine-text`, `zaroxi-core-workspace-*`.

- Domain: `zaroxi-domain-workspace`, `zaroxi-domain-buffer`, `zaroxi-domain-session`, `zaroxi-domain-project`, `zaroxi-domain-settings`, `zaroxi-domain-collaboration`, `zaroxi-domain-ai`, `zaroxi-domain-plugin`.

- Application: `zaroxi-application-workspace`, `zaroxi-application-editor`, `zaroxi-application-command`, `zaroxi-application-search`, `zaroxi-application-remote`, `zaroxi-application-plugin`.

- Interface: `zaroxi-interface-app`, `zaroxi-interface-desktop`, `zaroxi-interface-cli`, `zaroxi-interface-theme`.

- Infrastructure: `zaroxi-infrastructure-rpc`, `zaroxi-infrastructure-storage`, `zaroxi-infrastructure-http`, `zaroxi-infrastructure-logging`, `zaroxi-infrastructure-settings`, `zaroxi-infrastructure-tracing`, `zaroxi-infrastructure-process`, `zaroxi-infrastructure-ssh`.

- Intelligence: `zaroxi-intelligence-agent`, `zaroxi-intelligence-context`, `zaroxi-intelligence-planning`, `zaroxi-intelligence-embedding`, `zaroxi-intelligence-orchestrator`, `zaroxi-intelligence-eval`, `zaroxi-intelligence-memory`, `zaroxi-intelligence-tools`.

- Security: `zaroxi-security-audit`, `zaroxi-security-auth`, `zaroxi-security-policy`, `zaroxi-security-sandbox`, `zaroxi-security-validation`, `zaroxi-security-crypto`.

Quick notes for contributors
- Start small: pick a kernel/core crate and run its unit tests.
- Use `cargo test -p <crate-name>` to run tests for a single crate.
- When adding a crate, follow naming rules: use `zaroxi-{layer}-{name}` and update the workspace `Cargo.toml` and `docs/crates.md`.
- Prefer adding trait interfaces in core/domain and implementations in infrastructure.

Development commands
```bash
# Build workspace
cargo build --workspace

# Run tests for a crate
cargo test -p zaroxi-interface-desktop

# Run clippy and fmt checks
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

This document should be kept concise and updated as crates are added, removed, or renamed. When in doubt, follow the layer rules in `docs/architecture.md` and update this file to reflect changes.
