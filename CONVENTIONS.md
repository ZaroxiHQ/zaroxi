<!-- Rationale: shorten to a tight operational conventions file for Aider and humans. -->

# CONVENTIONS - Zaroxi Monorepo (operational)

## Source of truth
- Workspace layout: flat `crates/` plus composition roots in apps/, daemons/, harness.
- Dependency direction (strict): interface -> application -> domain -> core -> kernel.
- Infrastructure is a narrow edge adapter layer and must not own domain/core composition logic.

## Layer rules
- Outer layers may depend inward; inner layers must not depend outward.
- Composition roots may wire broader dependencies but must not become reusable libraries.

## Crate ownership rules
- Put composition helpers that coordinate domain/core ports in the owning application crate.
- Infra crates may implement allowed adapter ports but must remain narrow and not gain domain/core composition helpers.
- Extend existing crates first and avoid adding new crates unless necessary.
- Do not rename crates to hide dependency problems.
- Do not modify `scripts/architecture_check.sh` or add exceptions to make a change pass.

## Edit protocol
- Prefer small, file-scoped, additive edits targeting one concern per PR.
- Add tests when changing cross-crate contracts or behavior.
- Run the architecture check and affected cargo commands before finalizing changes.

## Stop conditions
- Stop and explain before editing if a change risks a layer violation, a dependency cycle, or crate ownership confusion.
- When stopped, provide a short explanation and a safe alternative (trait extraction, adapter placement, or design note).

## Token discipline
- Only load files you will edit and use `.aiderignore` to mask unrelated layers.
- Keep per-file edits small (~<250 LOC) and produce surgical diffs.
- If required context grows, stop and request a narrower file set.

## Validation checklist
- Run `./scripts/architecture_check.sh` and fix violations before committing.
- Verify modified crates with `cargo check -p <crate>` for directly affected crates.
- Confirm tests for changed behavior via `cargo test -p <crate>`.

<!-- End of concise conventions -->
