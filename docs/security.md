# Security — Model, Crate Responsibilities, and Current Posture

Purpose: Explain realistic security goals, which crates own which responsibilities, and what is implemented today versus planned hardening.

---

## Security goals

- Provide composable primitives for policy, audit, validation, and cryptography.
- Ensure security enforcement happens at clear boundaries (application/service boundary) with explicit checks and audit events.
- Avoid overstating guarantees — runtime enforcement and platform sandboxing are planned incrementally.

---

## Security crate family responsibilities

- `zaroxi-security-audit` — audit event model and recording helpers.
- `zaroxi-security-policy` — policy language and evaluation engine (decision checks).
- `zaroxi-security-auth` — authentication helpers and credential validation (minimal in current phase).
- `zaroxi-security-crypto` — small crypto utilities used by other crates.
- `zaroxi-security-sandbox` — sandbox helpers and adapters (platform-specific enforcement lives in infrastructure).
- `zaroxi-security-validation` — artifact validation and integrity checks.

---

## What is real today vs planned hardening

- Real today: policy primitives, audit models, validation helpers, and small crypto utilities usable by application and infrastructure code.
- Planned: robust OS sandbox profiles, integrated key management, enterprise authentication connectors, and signed update verification (Phase 9+).

---

## Runtime enforcement guidance

- Perform policy and permission checks at the application/service boundary before executing privileged operations.
- Emit audit events for sensitive operations (open/close workspace, apply patches, grant remote access).
- Do not assume client-side enforcement only — always validate inputs server-side or at the trusted application boundary.

---

## Developer checklist for security-sensitive changes

- Add policy checks for new privileged RPCs or CLI commands.
- Add structured audit events for operations that change workspace state or permissions.
- Add unit tests exercising validation and policy failure modes.

> This document describes the **security model** (which crates own what). To
> **report a vulnerability**, use the private process in
> [../.github/SECURITY.md](../.github/SECURITY.md) — do not open a public issue.

Related: [rpc.md](rpc.md) for validating remote requests, [architecture.md](architecture.md)
for enforcement boundaries, and [testing-and-quality.md](testing-and-quality.md)
for `cargo audit` / `cargo deny`.
