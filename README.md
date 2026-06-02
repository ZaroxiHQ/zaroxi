# Zaroxi Studio: AI-First IDE

> **Heavily under development** — APIs, features, and architecture may change.  
> Early adopters and contributors welcome; expect instability and breaking changes.

Repository: https://github.com/ZaroxiHQ/zaroxi

---

## What Zaroxi is

Zaroxi is a crate-first editor and IDE runtime written in Rust, organized into strict layers:

```
Kernel → Core → Domain → Application → Interface
```

with separate families for infrastructure, intelligence, and security. The repository hosts the working desktop harness used to exercise runtime behaviors.

---

## Current state

| Area | Status |
|------|--------|
| Architecture refactor (Phases 1–18) | Complete — desktop is a thin placement/render adapter |
| Desktop harness | Working — open/close, pending-close, command bar, GPU shell |
| Editor engine primitives | Implemented (in-memory buffers, transactions, view/projection) |
| AI panel content | Engine-owned, flows through unified `ShellWorkContent` carrier |
| Shared orchestration | 3 traits in `zaroxi-application-workspace::workspace_view` |
| Architecture check | 395 PASS, 0 FAIL |
| Disk-backed persistence | Planned |
| LSP baseline | Planned |
| AI edit/apply pipeline | Planned |

---

## Quick start

```bash
cargo build --workspace
cargo test -p zaroxi-interface-desktop
cargo run -p zaroxi-interface-desktop --bin gui_shell
bash scripts/architecture_check.sh
```

---

## Architecture at a glance

| Layer | Crates | Owns |
|-------|--------|------|
| Kernel | `zaroxi-kernel-*` | IDs, traits, math, primitives |
| Core | `zaroxi-core-*` | Engine composition, buffers, render pipeline |
| Domain | `zaroxi-domain-*` | Stable value objects, business models |
| Application | `zaroxi-application-*` | Orchestration traits, shared DTOs, action functions |
| Interface | `zaroxi-interface-*` | Desktop shell, GPU draw, layout, transcript |

**`ShellWorkContent`** is the single content carrier. Desktop action files are thin delegates to shared orchestration in `application-workspace`. See `docs/architecture.md` for the full contract.

---

## Key crates

- `zaroxi-interface-desktop` — start here to run the harness
- `zaroxi-application-workspace` — shared DTOs, traits, `build_work_content()`
- `zaroxi-core-engine-ui` — `ShellWorkContent`, `ContentView`, composer
- `zaroxi-core-editor-buffer` — in-memory buffer and transactions
- `zaroxi-domain-ai` — AI panel content models

---

## Docs

| File | Covers |
|------|--------|
| `docs/architecture.md` | Full architecture contract, content/action flow, shell geometry |
| `docs/crates.md` | Crate family guide and reading order |
| `docs/roadmap.md` | Phase-based delivery checkpoints |
| `docs/rpc.md` | RPC scaffold and status |
| `docs/security.md` | Security model and crate responsibilities |
| `docs/MISSING_FILES.md` | Documentation gaps and contributor tasks |

---

## Test and verification

```bash
cargo test --workspace
bash scripts/architecture_check.sh
cargo run -p zaroxi-interface-desktop --bin gui_shell
```
