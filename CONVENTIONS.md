CONVENTIONS - Zaroxi Monorepo
=============================

This file is a strict system-level prompt and policy document. It encodes architecture rules,
edit protocols, and LLM-safe behaviors. The content is authoritative and must be followed
by developers and by any automated assistant operating on the repository (Aider, CI bots, LLMs).

1) CLEAN ARCHITECTURE RULES (absolute)
-------------------------------------
- Dependency Direction (strict):
  interface  → intelligence → application → core → domain → kernel
  (outer layers may depend on inner layers, inner layers MUST NEVER depend on outer layers)

- Real examples (allowed vs forbidden):
  Allowed (interface depending inward):
    // In crate `zaroxi_interface_desktop` (outer layer)
    use zaroxi_application_workspace::ports::WorkspacePort;  // allowed

  Forbidden (inner layer depending outward):
    // In crate `zaroxi_domain_workflow` (domain layer)
    // NEVER import from application/intelligence/interface/core
    // DO NOT write code like:
    use zaroxi_application_workspace::service::Something; // FORBIDDEN

  Forbidden example of kernel depending on outer:
    // In crate `zaroxi_kernel_memory`
    use zaroxi_interface_desktop::ui::...; // NEVER

- NEVER rules (hard stops):
  - NEVER introduce dependencies from kernel/domain INTO core/application/intelligence/interface.
  - NEVER introduce cross-layer cyclic dependencies.
  - NEVER allow inner-layer crates to import symbols from outer layers.

2) ENFORCE ISOLATION
--------------------
- No cyclic dependencies:
  - Any proposed change that introduces a cycle must be rejected and explained.
- Cross-crate communication MUST go via traits:
  - Define a trait in the inner (lower) layer, and implement it in the outer layer.
  - Example pattern:
    - kernel/domain: define trait FooPort { ... }
    - application/core/interface: implement FooPort for ConcreteAdapter
  - Adapters and thin shims belong to the outer layer only.
- Kernel and Domain purity:
  - Kernel and domain crates must contain no side-effecting code tied to infrastructure.
  - Pure logic only: no tokio runtimes, no direct filesystem, no network clients.

3) CODING PROTOCOL FOR LLMs (editing rules)
-------------------------------------------
- Always produce unified diffs when editing code (git-compatible patch).
- Never refactor unrelated code in the same change.
- Keep any single file below 250 LOC (lines of code) in changes; if a file is larger,
  target a small, well-scoped chunk.
- Prefer additive changes over destructive ones:
  - Add new modules, traits, or adapters instead of changing existing contracts.
- Tests:
  - Propose or add unit tests for behavioral changes that cross crate boundaries.

4) SAFETY INTERRUPTS (automated checks)
---------------------------------------
If a proposed change risks any of the following:
  - circular dependency
  - layer violation (inner → outer import)
  - architectural drift (e.g. moving domain logic into interface)
→ STOP: do not apply the edit. Instead:
  - Explain the precise risk.
  - Propose a safe alternative (trait, adapter, or a PR-level design note).
  - If necessary, require a human architect sign-off.

5) TOKEN EFFICIENCY RULES (for tooling & LLMs)
----------------------------------------------
- Only load relevant crates for a task (use .aiderignore to mask others).
- Avoid scanning the whole repository. Use prefix-based masks (zaroxi-*/).
- Prefer surgical edits: change the smallest number of files with the least token cost.
- Keep prompts and repo-maps minimal and deterministic.
- Limit per-file context to the most relevant functions/signatures for the requested change.

6) RUST-SPECIFIC GUIDANCE
-------------------------
- Favor traits + dependency inversion:
  - Define interfaces (traits) in inner crates (kernel/domain).
  - Implement adapters in outer crates (application/infrastructure/interface).
- Avoid feature leakage across crates:
  - Do not enable crate-specific Cargo features in many dependents.
  - Prefer small, explicit feature flags contained and documented in the owning crate.
- Respect workspace boundaries:
  - Do not add path dependencies that break workspace isolation.
  - Use published crates only when appropriate; prefer internal traits/adapters.

7) CHANGE WORKFLOW (procedural)
-------------------------------
- Small, atomic PRs are required for architectural changes.
- When a change touches multiple layers, include a design note:
  - Which trait was added/changed
  - Why the adapter belongs in the outer layer
  - Backwards compatibility considerations
- For automated assistants:
  - If the assistant cannot confidently keep the change within these rules,
    it must STOP and emit a human-readable explanation and a proposed plan.

8) EXAMPLES (quick reference)
-----------------------------
- Adding a new persistence adapter:
  1. Define trait Persist in a domain or kernel crate.
  2. Implement the trait in an infrastructure crate (zaroxi-infrastructure-storage).
  3. Wire via constructor injection or a service-locator in the outer layer.
  4. Add tests in both trait crate (behavioral contract) and adapter crate.

- Fix that would violate rules (what to do instead):
  - If you are tempted to import application code from domain to reuse a util,
    instead:
      - Move the util to domain if it is domain logic, OR
      - Extract a small trait in domain and implement it in application/infrastructure.

9) ENFORCEMENT DISCIPLINE (for LLM and CI)
------------------------------------------
- CI must fail PRs that:
  - Add a dependency that violates the direction rules.
  - Introduce new path dependencies crossing layers.
- LLM assistants must run a lightweight static check:
  - Scan modified files' Cargo.toml for new dependencies.
  - If any new dependency originates from an outer layer, STOP and explain.

10) CONTACT / ESCALATION
------------------------
- For ambiguous design decisions, open an ARCHITECTURE-RFC in the repo:
  - Provide motivation, trade-offs, and a migration plan if needed.
- Architects must approve any exception to the "NEVER" rules.

Appendix: quick checklist for automated edits
---------------------------------------------
- [ ] Does the change add any crate dependencies? If yes, verify direction.
- [ ] Does the change modify signatures used across layers? If yes, add tests.
- [ ] Are edits additive? If not, justify and request human review.
- [ ] Is the file-level diff <= 250 LOC? If not, split the change.

This document is authoritative. Any assistant or developer failing to follow it must stop and request human review.
