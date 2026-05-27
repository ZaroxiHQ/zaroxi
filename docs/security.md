# Security — Current Crate Family and Posture

This document describes the security-related crates and the realistic, conservative security posture of the project before Phase 9.

Security crate family
- `zaroxi-security-audit` — Structured audit event model and helpers for recording security-relevant actions.
- `zaroxi-security-auth` — Authentication flows, token validation, and basic credential handling.
- `zaroxi-security-crypto` — Small cryptographic helpers and key handling primitives used by other crates.
- `zaroxi-security-policy` — Policy language, rule definitions, and evaluation engine for permission decisions.
- `zaroxi-security-sandbox` — Isolation primitives and sandbox-related helpers (platform adapters are scoped to infrastructure).
- `zaroxi-security-validation` — Artifact validation and integrity checks used during import/export and plugin validation.

Current foundations (conservative)
- The crates above provide primitive building blocks (types, evaluation engines, audit models) and a library of helpers used by infrastructure and application code.
- These crates intentionally avoid large platform integrations in the current phase: runtime enforcement hooks, OS sandbox profiles, and enterprise-grade identity providers are planned work.
- Permission checks and policy evaluation are implemented at the application layer for the flows currently exercised by the desktop harness. End-to-end policy enforcement and persistent policy stores are planned for later phases.

What to expect (limitations)
- The project provides policy primitives and audit models, but full, production-ready enforcement (OS sandbox profiles, signed updates, key management services) is planned and not yet shipped.
- Secrets handling and key management are minimally implemented for development; contributors should avoid committing secrets and follow repository guidance for secure handling in CI.

Developer guidance
- Use the `zaroxi-security-*` crates for modeling policies and performing validation checks in your code.
- Add audit events for sensitive operations (open/close workspace, apply patches, remote session grants).
- When adding new privileged RPCs or commands, perform explicit permission checks against the policy engine and emit audit events.

Roadmap alignment
- Phase 9/10: Strengthen enforcement of policies, add persistent policy stores and better integration with infrastructure adapters.
- Later phases: Harden sandbox and platform enforcement, add key management, and integrate with enterprise authentication providers.

Reporting and responsible disclosure
- Follow repository guidelines for reporting vulnerabilities. Avoid public disclosure before coordination with maintainers.

Summary
- The security crates provide conservative, well-scoped primitives and are intended as foundations. Runtime enforcement and production integrations are planned work and will be implemented incrementally starting in Phase 9 and beyond.
