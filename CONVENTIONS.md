# CONVENTIONS - Zaroxi Monorepo (authoritative, concise)

Rationale: replace duplicated / verbose content with a short, strict, machine-friendly conventions file that matches the current workspace architecture and helps automated assistants (Aider / CI) operate safely and efficiently.

## 1) Source-of-truth architecture
- Workspace layout: flat `crates/` plus app composition roots (apps/, daemons/, harness).
- Dependency direction (strict): interface -> application -> domain -> core -> kernel
  - Outer layers may depend inward; inner layers MUST NOT depend outward.
- Infrastructure is an edge adapter layer. It may implement small adapter ports but must not own domain/core composition logic.

## 2) Roles (short)
- interface: UI/presenters, composition adapters for UX; allowed to depend inward only.
- application: orchestrators / use-cases, composition of domain + core ports.
- domain: business logic and DTOs (pure, side-effect free).
- core: engine, editor primitives, low-level shared services.
- kernel: minimal, dependency-free primitives and traits.
- infrastructure: adapters for IO, persistence, network; keep narrow and adapter-focused.
- composition roots: binaries (apps/, daemons/, harness) may wire broader dependencies but must not become libraries other crates depend on.

## 3) Ownership guidance (recent fixes)
- Put in-memory workspace/buffer composition helpers in `zaroxi-application-workspace`, not `zaroxi-infrastructure-memory`.
- `zaroxi-infrastructure-memory` should focus on history/durability/checkpoint responsibilities only.
- Prefer extending existing crates before adding new crates.

## 4) Forbidden/Stop conditions
Stop and explain if a change:
- Introduces inner -> outer import (layer violation).
- Introduces a cyclic dependency.
- Moves domain/core composition helpers into an infra crate.
If any of the above is possible, do not edit; produce a human-readable explanation and a safe alternative (trait extraction, adapter placement, or RFC).

## 5) Editing rules for humans and automated assistants
- Make small, atomic changes. Target a single logical concern per PR.
- Prefer additive edits (new trait, new adapter) over changing upstream contracts.
- Add unit tests when modifying cross-crate contracts or behavior.
- Do not modify `scripts/architecture_check.sh` or add exceptions to make a change pass; fix the underlying ownership/direction instead.
- Do not rename crates to hide dependency direction problems.

## 6) Token-efficiency guidance for Aider
- Only load files you intend to edit.
- Use `.aiderignore` to mask unrelated layers.
- Avoid loading the entire repo context; prefer a minimal set of crates.
- Produce small diffs; keep per-file edits below ~250 LOC.
- If context becomes large or ambiguous, STOP and request a narrower set of files.

## 7) Short Zaroxi-specific examples
- Correct: implement DurabilityRepository in `zaroxi-infrastructure-memory` and use it from `zaroxi-application-workspace`.
- Incorrect: moving in-memory buffer composition helpers from application -> infra to satisfy a dependency (do not do this).
- Correct: define trait in domain/core and implement it in infra or application, wired by composition at apps/.

## 8) CI / LLM checks
- CI will fail PRs that add forbidden dependencies or path deps crossing layers.
- Automated assistants must run a lightweight dependency-family check before proposing edits.

## 9) Contact / escalation
- For ambiguous design decisions or exceptions, open an ARCHITECTURE-RFC describing:
  - The change, motivation, alternatives, and migration plan.
  - Who should review/approve the exception.

---

This file is the single authoritative conventions document for repo edits. Follow it strictly and stop for human review on any architecture-risking change.
