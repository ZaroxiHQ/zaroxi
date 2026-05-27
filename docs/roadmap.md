# Roadmap — From Current State Toward Phase 14

This roadmap starts from the current, verified pre-Phase-9 state and lays out concrete next phases. It is intentionally pragmatic: each phase lists specific deliverables and keeps the distinction between implemented and planned work clear.

Current (pre-Phase-9)
- Rust-native, crate-based editor shell and desktop harness (`zaroxi-interface-desktop`, `zaroxi-interface-app`) are working and used to exercise runtime flows.
- Core editor primitives, engine subsystems, and the GPU shell are present and under test.
- AI explanation/projection is implemented (editor-side projection), but full AI edit/apply flow is not yet implemented.
- Desktop/app tests for the harness path are passing.
- Real disk-backed workspace persistence and full LSP integration are not yet implemented.

Next (Phase 9) — Disk-backed workspaces (priority)
Goal: Implement reliable disk-backed workspace and file persistence so the editor can operate on real file trees and durable buffers.
Deliverables:
- Storage adapter implementations for local filesystem (infrastructure-storage) with crash-safe write semantics.
- Integration: wire `zaroxi-application-workspace` and `zaroxi-domain-buffer` to persistent storage.
- Migration tests and end-to-end harness validation using the desktop harness.
- Backup and checkpoint persistence strategy (safe restore on crash).

Phase 10 — LSP baseline
Goal: Provide a stable LSP integration baseline for language features and diagnostics.
Deliverables:
- LSP client scaffolding and session manager (`zaroxi-core-platform-lsp` / `zaroxi-infrastructure-*` where necessary).
- Diagnostics plumbing into editor UI and workspace indexing.
- Tests and sample integrations for Rust and one additional language (e.g., TypeScript).

Phase 11 — AI edit/apply
Goal: Ship a safe AI edit/apply workflow that supports preview, verification, and explicit user confirmation before applying changes.
Deliverables:
- AI apply pipeline with preview and verification (`zaroxi-intelligence-*` + patch-engine integration).
- Safety checks and validation (`zaroxi-security-validation`) before any automatic apply.
- UI/UX flows in the desktop harness for preview and user confirmation.

Phase 12 — Performance and rendering
Goal: Improve performance across rendering, workspace operations, and large-file handling.
Deliverables:
- Incremental rendering and layout improvements in `zaroxi-core-engine-*`.
- Workspace indexing and search performance improvements.
- Memory and CPU profiling with actionable optimizations.

Phase 13 — Productization
Goal: Prepare the codebase for wider distribution and early adopters.
Deliverables:
- Packaging and installers for target platforms.
- Update & installer signing, release pipelines, and reproducible build documentation.
- Documentation and contributor onboarding improvements.

Phase 14 — Alpha release
Goal: Publish an alpha release that includes the core editor experience, persistent workspaces, LSP baseline, and safe AI apply flows.
Deliverables:
- Public alpha release artifacts and release notes.
- Early user feedback channels and tracked issues.

Later work (post-alpha)
- Collaboration & multi-user editing.
- Advanced AI features (fine-tuning, local model support, agent orchestration).
- Enterprise features and integrations.

How to contribute to the roadmap
- Open an issue labeled `roadmap` with a clear proposal and implementation plan.
- For feature work: open a design doc and link to relevant crates and tests.
- Keep docs/roadmap.md and docs/architecture.md synchronized when plans shift.

Success criteria for Phase 9
- Desktop harness exercises real disk-backed open/save flows in CI.
- Tests cover storage, checkpoint, and recovery flows.
- No regressions in desktop/app harness tests.

This roadmap is purposely concrete and conservative; it reflects the repository's current state and the next practical milestones toward a durable editor product.