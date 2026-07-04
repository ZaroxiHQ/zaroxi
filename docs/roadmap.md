# Roadmap — Execution Plan From Current State (Post-Phase-18)

Purpose: Concrete, delivery-oriented checkpoints from the project's current state. Each phase lists deliverables and measurable success criteria.

---

## Where we are now

- The desktop is a thin placement/render adapter; shared orchestration lives in
  `zaroxi-application-workspace` behind a small set of traits.
- `ShellWorkContent` is the single content carrier for editor, explorer,
  terminal, and AI panels (see [runtime-and-rendering.md](runtime-and-rendering.md)).
- Working desktop harness: open/close, pending-close, command bar, AI projection.
- Cross-platform CI (Linux/macOS/Windows) with architecture, security, and
  supply-chain gates all green.
- Disk-backed persistence, an LSP client, and a full AI apply pipeline are not
  yet implemented.

For architecture context see [architecture.md](architecture.md); for the current
AI state see [ai-and-editor-flows.md](ai-and-editor-flows.md).

---

## Phase 19 — Disk-backed workspaces

Deliverables:
- `zaroxi-infrastructure-storage` local FS adapter with crash-safe write semantics.
- Wire `zaroxi-application-workspace` and `zaroxi-domain-buffer` to persistent storage.
- Migration and recovery tests exercised by the desktop harness.

Success criteria:
- Harness CI exercises open/save and checkpoint restore on real disk-backed workspaces.
- No regressions in harness tests.

---

## Phase 20 — LSP baseline

Deliverables:
- LSP client scaffolding and session manager.
- Diagnostics plumbing into editor UI and workspace indexing.
- Tests and example integrations.

Success criteria:
- LSP sessions can be started/stopped in the harness and diagnostics surface in editor views.

---

## Phase 21 — AI edit/apply

Deliverables:
- AI apply pipeline: produce patches, preview UI, verification and user confirmation.
- Safety checks via `zaroxi-security-validation` and audit emissions.

Success criteria:
- Harness tests cover preview/verify/apply flows and policy failures.

---

## Later phases (22–24)

- Phase 22: Performance and incremental rendering improvements.
- Phase 23: Packaging, installers, and release infrastructure.
- Phase 24: Public alpha release with core editor, persistence, LSP baseline, and safe AI apply.

---

## How to contribute to the roadmap

- Open an issue with a clear implementation plan and targeted crates.
- For new panels: follow the pattern in `docs/architecture.md` (add field to `ShellWorkContent`, wire through `build_work_content()`).
- For new commands: add label to `command_bar_labels()`, match arm in `execute_command_by_index()`.
- Small PRs that add tests or per-crate READMEs are high-value early contributions.
