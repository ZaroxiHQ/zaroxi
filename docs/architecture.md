# Zaroxi — Complete Architecture Specification (v1.0)

This document is the authoritative architecture reference for Zaroxi: an AI‑first, GPU‑accelerated IDE.
It defines the layered crate layout, naming and dependency rules, enforcement tooling, and a practical migration plan.

Table of contents
- Short summary
- Crate hierarchy (by layer)
- Renames, new crates, removed/merged crates
- Dependency rules & graph
- Per-crate responsibilities and allowed dependencies
- Enforcement & CI
- Migration plan
- When to split crates
- Appendices

---

## Short summary of immediate changes

1. Create a small set of kernel-* crates that own low-level primitives (IDs, time, memory, traits).
2. Split existing monoliths into clear kernel/core/domain/application/interface responsibilities and create stubs for any missing crates.
3. Enforce strict dependency rules via CI scripts and cargo-deny; prevent upward or cross-layer dependencies and cycles.

---

## Crate hierarchy (top → bottom)

ASCII overview:

interface
  ↑
application
  ↑
domain
  ↑
core
  ↑
kernel

Special namespaces:
- intelligence (may depend on kernel, core, domain)
- security (may depend on kernel, core, domain)
- infrastructure (may depend on kernel, core only)

Canonical crate groups (representative, not prescriptive):

- Kernel (zaroxi-kernel-*)
  - zaroxi-kernel-core, zaroxi-kernel-types, zaroxi-kernel-errors, zaroxi-kernel-memory,
    zaroxi-kernel-async, zaroxi-kernel-time, zaroxi-kernel-math, zaroxi-kernel-collections,
    zaroxi-kernel-traits, zaroxi-kernel-config

- Core (zaroxi-core-*)
  - Input, event, commands, runtime, scheduler, task, threading, text*, render*, layout, ui

- Domain (zaroxi-domain-*)
  - Editor domain, workspace, project, buffer, selection, history, ai, settings

- Application (zaroxi-application-*)
  - Editor orchestration, workspace orchestration, search, AI orchestration, navigation, refactor

- Interface (zaroxi-interface-*)
  - App shell, concrete editor UI, GUI/CLI entrypoints, theming

- Intelligence (zaroxi-intelligence-*)
  - Agent runtime, planning, memory, context, orchestrator, embeddings

- Infrastructure (zaroxi-infrastructure-*)
  - RPC, HTTP, storage, settings, logging, metrics, tracing, network adapters

- Security (zaroxi-security-*)
  - Sandbox, policy, validation, auth, audit, crypto

- Platform (zaroxi-platform-*)
  - LSP, syntax/grammar handling, debugger, git integrations, plugin host contracts

Note: keep crates small and focused; favor many small crates with clear public APIs over large monoliths.

---

## Renames, new crates, and splits

Planned rename mappings (apply consistently and update workspace members):

- zaroxi-engine-* → zaroxi-core-*
- zaroxi-service-* → zaroxi-application-*
- zaroxi-infra-* → zaroxi-infrastructure-*
- zaroxi-ai-agent → zaroxi-intelligence-agent
- zaroxi-domain-ai-context → zaroxi-domain-ai
- zaroxi-lang-* → zaroxi-platform-*
- zaroxi-ops-* → zaroxi-workspace-*
- zaroxi-foundation → zaroxi-kernel-core
- zaroxi-app → zaroxi-interface-app

Split patterns: editor monoliths become:
- zaroxi-interface-editor (UI)
- zaroxi-core-text (core text primitives)
- zaroxi-domain-buffer (domain buffer semantics)

New crates to add (examples): zaroxi-kernel-memory, zaroxi-core-render-graph, zaroxi-intelligence-orchestrator, zaroxi-infrastructure-tracing, zaroxi-interface-editor, tools/crate-lint.

Removed/merged: any monoliths that mix UI, domain, infra responsibilities should be split per the plan.

---

## Dependency rules (explicit)

Allowed dependency directions (strict):
- kernel → kernel | external
- core → kernel | core
- domain → kernel | core | domain
- application → kernel | core | domain | application
- interface → kernel | core | domain | application | interface
- intelligence → kernel | core | domain
- security → kernel | core | domain
- infrastructure → kernel | core

Forbidden:
- Any crate depending on a higher-level layer (e.g., core → domain) is forbidden.
- Infrastructure must not depend on domain/application/interface.
- No circular dependencies across any crates.

Examples:
- Allowed: zaroxi-core-render → zaroxi-kernel-math
- Forbidden: zaroxi-core-render → zaroxi-domain-buffer (core must not depend on domain)

---

## Dependency graph & enforcement

- The canonical dependency graph is enforced by:
  - .github/scripts/check_layer_deps.py — strict layer checks, report mode, and fix suggestions
  - .github/scripts/check_crate_naming.py — crate name validation
  - .github/scripts/check_circular_deps.py — cycle detection
  - .github/scripts/check_crate_size.py — size guards for maintainability
  - deny.toml — cargo-deny policy for licenses, advisories, duplicate versions, and additional bans

- CI runs these checks on each PR; maintainers should enable branch protection requiring these checks to pass before merge:
  - Required status checks: check (Rust CI matrix), layer-deps (artifact), naming-convention, circular-deps, crate-size, deny

- How to handle a violation:
  1. If a crate depends on a higher layer, refactor the required functionality into a crate at an allowed layer (e.g., core or kernel) and expose a small interface.
  2. If behavior belongs to the higher layer semantically, move the caller to the higher layer instead.
  3. For exceptions, open an issue and request an explicit CI exception (rare).

---

## Per-crate responsibilities (summary)

For each layer, document the responsibilities and the exact allowed dependency targets. Keep these documents next to the crate (README.md) and update when rules change. The architecture enforcement scripts read naming conventions to determine layer membership.

---

## When to split a crate

Heuristics:
- Lines of Rust source > 1500 — consider splitting (CI emits [WARN]).
- Logical separation of concerns (UI vs domain vs infra) — split along layer boundaries.
- Growing dependency set that crosses layer boundaries — split to keep boundaries clean.
- If a crate contains unrelated features (rendering + domain models), extract coherent modules into separate crates.

---

## Migration plan (practical)

Phase 0 — Preparation
- Add this document to repo, add tools/crate-lint, and pin workspace Cargo.toml with explicit members.
- Run the enforcement scripts in local mode (just arch) to collect violations.

Phase 1 — Kernel & Core skeleton
- Create kernel-* crates and minimal exports.
- Create core-text-rope and core-text stubs.

Phase 2 — Domain split
- Move pure logic into domain-* crates; update callers in application/interface layers.

Phase 3 — Application & Intelligence
- Create application-* and intelligence-* crates, move orchestration code.

Phase 4 — Platform & Infrastructure
- Add platform-* and infrastructure-* adapters; keep trait-only interfaces in core/domain.

Phase 5 — Security & Collaboration
- Implement sandbox and CRDT backends behind interfaces.

Phase 6 — Validation
- Run full CI, fix violations, and iterate until clean.

Notes:
- Make small, incremental PRs; each PR should compile and pass tests.
- Use deprecation windows for large moves; leave shim crates that re-export old APIs temporarily.

---

## Enforcement tooling summary

Scripts live in .github/scripts/ (stdlib Python only):
- check_layer_deps.py — strict layer matrix enforcement, JSON report, fix suggestions
- check_crate_naming.py — validates crate naming pattern and core sublayers
- check_circular_deps.py — detects cycles in internal crate graph
- check_crate_size.py — counts lines and enforces warn/fail thresholds

CI:
- Runs the Rust matrix (check/clippy/test/fmt/deny) and the architecture scripts in parallel.
- Uploads layer-deps JSON report as artifact for easy inspection.

Local dev flow:
- Use justfile helpers:
  - just arch — run architecture scripts locally
  - just arch-report — create JSON report and print via jq (if available)
  - just size — crate size check
  - just validate — full local pre-push gate (arch + deny + fmt + clippy)

---

## Appendices

Appendix A — Naming rules (summary)
- Crate names must follow: zaroxi-{layer}-{...}
- Core crates must follow: zaroxi-core-{sublayer}-{concern}
- Use check_crate_naming.py to validate and catch duplicates.

Appendix B — Examples & remediation patterns
- If core crate A needs functionality in domain crate B, either:
  - Move that functionality down into core (preferred), or
  - Move the consumer into application/domain layer where allowed.

Appendix C — Governance
- Maintain workspace Cargo.toml with explicit members (no globs).
- Update CI scripts when adding new namespaces or allowed exceptions.
- Open an issue and obtain maintainer approval for any architecture exception.

---

If you'd like, I can:
- Create a GitHub-friendly summarized README from this doc
- Generate the initial empty kernel-* and core-* crate stubs
- Run the enforcement scripts locally (in CI) and produce the first reports

Tell me which of the above you'd like me to apply next.
