# Documentation Gaps and Next Tasks

This file lists small, concrete documentation tasks that should be completed next. Each item is actionable and scoped to the current pre‑Phase‑9 codebase.

1. Per-crate quickstarts
- Create a 1‑page `README.md` for the following crates with: purpose, how to run tests, and a short usage example where applicable:
  - `zaroxi-interface-desktop`
  - `zaroxi-application-workspace`
  - `zaroxi-core-editor-buffer`
  - `zaroxi-core-engine-render`
  - `zaroxi-infrastructure-rpc`
  - `zaroxi-intelligence-agent`
- Target: one short README per crate (3–8 lines + commands).

2. Storage / Phase 9 docs
- Add a storage design doc describing the planned disk-backed buffer model and adapter hooks in `docs/storage.md` (draft). Keep it brief and tied to `zaroxi-infrastructure-storage` and `zaroxi-application-workspace`.

3. API / Rustdoc publication
- Establish a `docs/api/` generation step that runs `cargo doc --workspace` in CI and publishes artifacts or makes them available for local developers.

4. Docs CI link-checker
- Add a lightweight CI workflow to validate internal markdown links in `docs/` on PRs and pushes (this repo now includes a simple workflow under `.github/workflows/docs-link-check.yml`).

5. Remove remaining non-doc Tauri references
- Non-doc occurrences of `Tauri` remain in a few build or comment files; a focused cleanup has been applied to the most visible ones (`.gitignore`, `flake.nix`, a syntax cache comment). If you want full removal across the codebase, run a small follow-up PR to replace or annotate remaining references in non-doc build files and source comments.

6. Contributor onboarding (docs/contributing-docs.md)
- Add a short docs-maintainer checklist: how to update `docs/crates.md` and `docs/architecture.md` when crates are added, and how to run the link-check workflow locally.

7. Test & examples
- Add tiny examples (in `examples/`) demonstrating: opening a workspace in the harness, running the desktop harness in debug, and a small test case showing checkpoint restore flow.

Priority and ownership
- High priority: Per-crate quickstarts (1) and Docs CI link-checker (4).
- Medium priority: Storage/Phase9 docs (2) and API publication (3).
- Low priority: Examples (7) and contributor docs checklist (6).

How to contribute
- Open a small PR that adds one per-crate README at a time and update `docs/crates.md` with the new shortlink.
- Reference this file (`docs/GAPS.md`) in your PR description for traceability.
