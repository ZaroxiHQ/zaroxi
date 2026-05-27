# Roadmap — Execution Plan From Current State (Pre‑Phase‑9)

Purpose: Provide concrete, delivery‑oriented checkpoints from the project's current state toward Phase 14. Each phase lists deliverables and measurable success criteria.

---

## Where we are now

- Working desktop harness that exercises runtime flows and GPU shell rendering.
- Core editor primitives and engine subsystems are implemented and under test.
- AI explanation/projection exists; full AI edit/apply is not implemented.
- Disk-backed persistence and LSP baseline are not implemented yet.

---

## Phase 9 — Disk-backed workspaces (priority)

Deliverables:
- `zaroxi-infrastructure-storage` local FS adapter with crash-safe write semantics.
- Integration: wire `zaroxi-application-workspace` and `zaroxi-domain-buffer` to persistent storage.
- Migration and recovery tests exercised by the desktop harness.

Success criteria:
- Desktop harness CI exercises open/save and checkpoint restore on real disk-backed workspaces.
- No regressions in harness tests.

---

## Phase 10 — LSP baseline

Deliverables:
- LSP client scaffolding and session manager in `zaroxi-core-platform-lsp` or equivalent.
- Diagnostics plumbing into editor UI and workspace indexing.
- Tests and example integrations for Rust and one other language.

Success criteria:
- LSP sessions can be started/stopped in the harness and diagnostics surface in editor views.

---

## Phase 11 — AI edit/apply

Deliverables:
- AI apply pipeline: produce patches, preview UI, verification and user confirmation flows.
- Safety checks via `zaroxi-security-validation` and audit emissions.

Success criteria:
- Harness tests cover preview/verify/apply flows and policy failures.

---

## Later phases (12–14)

- Phase 12: Performance and incremental rendering improvements.
- Phase 13: Packaging, installers, and release infrastructure.
- Phase 14: Public alpha release with core editor, persistence, LSP baseline, and safe AI apply.

---

## Not a priority right now

- Full multi-node collaboration (real-time multi-user editing) — low priority before persistence and LSP.
- Enterprise identity connectors and signed-update systems — planned after Phase 9–11 foundations.

---

## How to contribute to the roadmap

- Open an issue labeled `roadmap` with a clear implementation plan and targeted crates.
- Small PRs that add tests or per-crate README are high-value early contributions.

Related: `docs/MISSING_FILES.md` outlines immediate documentation and small engineering tasks contributors can pick up.
