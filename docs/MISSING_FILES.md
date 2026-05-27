# Documentation & Architecture Gaps (current repository)

Purpose: A precise, actionable list of missing documentation or small architecture gaps focused on the current pre‑Phase‑9 codebase. Use this file to pick small contributor tasks.

---

High-priority (small, high-value tasks)

- Per-crate quickstarts: add a 1-paragraph `README.md` for these crates with purpose + how to run tests:
  - `zaroxi-interface-desktop`
  - `zaroxi-application-workspace`
  - `zaroxi-core-editor-buffer`
  - `zaroxi-core-engine-render`
  - `zaroxi-infrastructure-rpc`
  - `zaroxi-intelligence-agent`

- Docs CI: ensure the existing `.github/workflows/docs-link-check.yml` runs and validate links in `docs/` on PRs.

Medium-priority (design & infra)

- Storage design note: `docs/storage.md` (short draft) describing the planned disk-backed buffer model and where adapters will be added.
- API publishing: CI step to run `cargo doc --workspace` and publish or stage docs for maintainers.

Low-priority (examples & hygiene)

- Tiny examples under `examples/` that demonstrate: opening a workspace, running the desktop harness, and a checkpoint restore test-case.
- Per-crate `README.md` additions should be concise (one-line purpose, one build/test command).

---

Retention rationale

This file replaces older, informal gap notes and is intentionally scoped to the *current repository state*. It does not reference older UI frameworks or deprecated architecture.

If you implement one of the items above, update this file and `docs/crates.md` as needed.
