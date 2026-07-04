# Zaroxi Studio Documentation

Documentation for [Zaroxi Studio](../README.md) — a native, GPU-rendered,
pure-Rust editor and IDE runtime.

## Start here

| You are… | Read |
|---|---|
| **New to the project** | [system-context.md](system-context.md), then the root [README](../README.md) |
| **A contributor setting up** | [development.md](development.md) |
| **Reading the architecture** | [architecture.md](architecture.md) → [workspace-structure.md](workspace-structure.md) |
| **Working on the desktop/UI** | [runtime-and-rendering.md](runtime-and-rendering.md) |
| **Working on AI/editor features** | [ai-and-editor-flows.md](ai-and-editor-flows.md) |
| **Reviewing CI / quality gates** | [testing-and-quality.md](testing-and-quality.md) |
| **Making a design decision** | [decisions/](decisions/) (ADRs) |

## Map

**Architecture**
- [architecture.md](architecture.md) — flagship: layers, dependency rules, runtime shape
- [system-context.md](system-context.md) — what the system is, users, external deps, boundaries
- [workspace-structure.md](workspace-structure.md) — monorepo layout, crate families, naming, placement
- [runtime-and-rendering.md](runtime-and-rendering.md) — GUI shell, event loop, rendering, content/action flow
- [ai-and-editor-flows.md](ai-and-editor-flows.md) — how AI integrates with the editor
- [decisions/](decisions/) — Architecture Decision Records

**Contributing & operations**
- [development.md](development.md) — local setup, build/run/test, CI helpers
- [testing-and-quality.md](testing-and-quality.md) — CI layout, architecture enforcement, audit/deny
- [crates.md](crates.md) — crate catalog and where to start
- [roadmap.md](roadmap.md) — delivery direction

**Reference**
- [security.md](security.md) — security model and crate responsibilities
- [rpc.md](rpc.md) — RPC surface and status

**Root-level policies**
- [Contributing](../CONTRIBUTING.md) · [Code of Conduct](../CODE_OF_CONDUCT.md) · [Security policy](../.github/SECURITY.md) · [License](../LICENSE)

## Conventions

- Docs are **docs-as-code**: they live beside the code and are reviewed in PRs.
- Each document owns one topic; cross-link instead of duplicating.
- Keep statements grounded in the current codebase. When code changes a
  documented contract, update the doc in the same PR.
