# RPC — Current Status and Realistic Near-Term Direction

Purpose: Describe the present RPC surface, what is scaffolding versus production-ready, and the relationship between `application-remote`, `infrastructure-rpc`, and transport adapters.

---

## What exists today

- `zaroxi-infrastructure-rpc` provides transport adapters and test utilities (in-memory transports, socket-based test harnesses).
- `zaroxi-application-remote` consumes RPC adapters to present a remote workspace abstraction to the application layer.
- The current implementation is conservative and aimed at local orchestration and testability rather than production remote deployment.

---

## Scaffolding vs production readiness

- Scaffolding (present): in-memory transports, JSON-based framing for readability in tests, socket adapters for local integration.
- Not production-ready (planned): hardened transports (TLS/mTLS), protocol versioning, service discovery, and binary framing for performance.

---

## Crate relationships and responsibilities

- `zaroxi-infrastructure-rpc` (infrastructure): implements concrete transports and test helpers. It depends on trait/protocol definitions from core/domain.
- `zaroxi-application-remote` (application): maps remote RPC calls into application-level workspace/session abstractions and applies domain operations when authorized.
- Network, process, and SSH adapters live under `zaroxi-infrastructure-*` families (e.g., `zaroxi-infrastructure-ssh`, `zaroxi-infrastructure-process`), and should implement the same conservative contract model.

---

## Realistic near-term direction

- Phase 9: RPC remains useful for remote orchestration; storage/persistence reduces some local RPC usage but remote-backed workspaces remain an intended use-case.
- Phase 10–12: Incrementally harden transports: add TLS support, authentication integration, protocol versioning, and production-grade error handling.

---

## Practical contributor guidance

- Prefer adding RPC message definitions in a protocol/core crate; keep transport adapters thin and well-tested.
- Use in-memory transports from `zaroxi-infrastructure-rpc` for unit and integration tests.
- For new RPC entrypoints: add protocol types, a handler stub in application, and tests that exercise validation and authorization.

Related: `docs/crates.md` (which crates own RPC bits) and `docs/security.md` for notes on authentication and validation.
