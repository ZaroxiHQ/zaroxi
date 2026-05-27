# Crates Guide — Responsibilities and Where To Start

Purpose: A contributor-focused, concise map of the crate ecosystem and guidance on which crates to read and modify for specific kinds of work. This document is deliberately not an architecture narrative — it is a practical crate catalog and onboarding path.

---

## Naming conventions

- Crates follow: `zaroxi-{layer}-{name}` where `{layer}` is one of: `kernel`, `core`, `domain`, `application`, `interface`, `infrastructure`, `intelligence`, `security`.
- Keep crate responsibilities narrow: prefer many small, focused crates to large monoliths.

---

## Crate families (grouped)

- Kernel (foundational): `zaroxi-kernel-*` — tiny types, IDs, and traits.
- Core (engine/runtime): `zaroxi-core-*` — buffers, render engine, input, scheduler, workspace helpers.
- Domain (business logic): `zaroxi-domain-*` — workspace model, buffer semantics, session lifecycle.
- Application (feature composition): `zaroxi-application-*` — workspace orchestration, command routing, editor composition.
- Interface (product shells): `zaroxi-interface-*` — desktop harness, CLI, theme assets.
- Infrastructure (adapters): `zaroxi-infrastructure-*` — storage, RPC, HTTP, process, tracing, settings.
- Intelligence (AI tooling): `zaroxi-intelligence-*` — agent runtime, planning, memory, embedding helpers.
- Security (policy & audit): `zaroxi-security-*` — policy evaluation, audit models, crypto helpers.

---

## Start-here crates (recommended for new contributors)

- `zaroxi-interface-desktop` / `zaroxi-interface-app` — run the harness and reproduce runtime flows.
- `zaroxi-application-workspace` — central integration point for workspace lifecycles.
- `zaroxi-core-editor-buffer` — critical for understanding buffer semantics and transaction models.
- `zaroxi-core-engine-render*` — GPU shell and render pipeline primitives (useful for UI/rendering work).

---

## Foundational vs feature-facing crates

- Foundational (change with care): Kernel and Core crates (e.g., `zaroxi-kernel-*`, `zaroxi-core-*`). Changes here affect many crates.
- Feature-facing (safer to iterate): Application, Interface, Intelligence, Infrastructure crates. These compose or implement contracts defined by core/domain.

---

## How to choose the right crate for new work

- If you change a type or trait used across the codebase, prefer adding a new trait in Core/Domain and an implementation in Infrastructure.
- For UI/UX or harness changes, work in `zaroxi-interface-*` and `zaroxi-application-*`.
- For AI features that do not apply side-effects directly, implement logic in `zaroxi-intelligence-*` and return plans/patches to the application.

---

## Quick commands for crate work

```bash
# Run tests for a single crate
cargo test -p zaroxi-interface-desktop

# Build a crate
cargo build -p zaroxi-application-workspace

# Run clippy on workspace
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Maintenance notes

- When adding/removing/renaming crates: update `Cargo.toml` workspace entries and this file (`docs/crates.md`).
- Keep entries short and concrete — add a one-line purpose and one-line "start here" command to per-crate READMEs when possible.

Related: `docs/architecture.md` (system boundaries) and `docs/MISSING_FILES.md` (current documentation gaps).
