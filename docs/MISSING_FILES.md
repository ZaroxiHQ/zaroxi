# Documentation & Architecture Gaps (current repository)

Purpose: A precise, actionable list of missing documentation or small architecture gaps. Use this file to pick small contributor tasks.

---

## High-priority (small, high-value tasks)

- Per-crate quickstarts: add a 1-paragraph `README.md` for these crates with purpose + how to run tests:
  - `zaroxi-application-workspace` — shared orchestration and DTOs
  - `zaroxi-interface-desktop` — desktop harness integration
  - `zaroxi-core-engine-ui` — `ShellWorkContent`, `ContentView`
  - `zaroxi-domain-ai` — AI panel content models

- Docs CI: ensure the existing `.github/workflows/docs-link-check.yml` runs and validate links in `docs/` on PRs.

## Medium-priority (design & infra)

- Storage design note: `docs/storage.md` (short draft) describing the planned disk-backed buffer model and where adapters will be added.
- API publishing: CI step to run `cargo doc --workspace` and publish or stage docs for maintainers.

## Low-priority (examples & hygiene)

- Tiny examples under `examples/` that demonstrate: opening a workspace, running the desktop harness, and a checkpoint restore test-case.
- Per-crate `README.md` additions should be concise (one-line purpose, one build/test command).

---

## Completed

- `docs/architecture.md` — rewritten post-Phase-18 with full layer model, content/action contracts, shell geometry, and trait reference.
- Architecture check script (`scripts/architecture_check.sh`) — actively enforces layer boundaries (395 PASS, 0 FAIL).

---

If you implement one of the items above, update this file and `docs/crates.md` as needed.
