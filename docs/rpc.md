# RPC (Remote Procedure Call) — Current Role and Status

Overview
- The repository contains `zaroxi-infrastructure-rpc` as the transport/adapters namespace and `zaroxi-application-remote` for remote workspace orchestration.
- At present the RPC crates provide a well-defined scaffold and some adapters, but the RPC surface is intentionally conservative: much of the remote orchestration and production-grade transport integration is planned for later phases.

Current position
- `zaroxi-infrastructure-rpc` implements transport primitives and testing harnesses used by infra-level components. It provides in-memory and socket-based transports useful for tests and local orchestration.
- `zaroxi-application-remote` sits at the application layer and consumes RPC adapters to present a remote workspace abstraction to the application. It is intended to be the integration point for remote sessions and remote-backed workspaces.
- In the current pre-Phase-9 state RPC is used for some local orchestration and testing harnesses, but there is not yet a fully hardened, production remote platform running across separate machines.

Design notes
- RPC transports are implemented as adapters behind trait contracts. This keeps the core protocol and message definitions independent of transport choices.
- Supported transports (examples): in-memory channels (for tests), Unix domain sockets, and TCP sockets (adapter available). Transport-specific adapters live under `zaroxi-infrastructure-rpc`.
- Message framing uses a conservative JSON-based wire format for readability in tests and debugging. The design allows alternative binary formats in the future.

Practical guidance for contributors
- If you need to add a new RPC method, follow these steps:
  1. Add the message type or method definition to the protocol crate or core/message crate used by RPC.
  2. Add a handler stub in the application layer (`zaroxi-application-remote` or the specific application consumer).
  3. Add tests using in-memory transports from `zaroxi-infrastructure-rpc` test utilities.

Security & validation
- Authentication and authorization checks are enforced at the application/service boundary (permission checks should be performed before executing privileged operations).
- RPC stubs include parameter validation helpers. For any new RPC entrypoints, validate inputs and add tests exercising validation paths.

Roadmap alignment
- Phase 9: Storage and persistence will reduce reliance on RPC for local workspace persistence; RPC will remain the primary remote orchestration mechanism for remote-backed workspaces.
- Later phases (10–12): Harden transports (TLS/mTLS), add service discovery, add protocol versioning and binary framing options for performance-sensitive paths.

Summary
- RPC crates are present and provide a usable scaffold and in-process/in-memory utilities for tests.
- Full production remote deployment and hardened transports are planned, and contributions are welcome that keep the protocol stable and well-tested.
