# Architecture — System Responsibilities and Runtime Flow

Purpose: Describe the real, current architecture and runtime flow for Zaroxi (pre‑Phase‑9). This file explains boundaries, request/data/control flow, where infrastructure/intelligence/security belong, and how the desktop harness and GPU shell are exercised.

---

## Architecture principles

- Strict layer separation: Kernel → Core → Domain → Application → Interface. Lower layers must not depend on higher layers.
- Small, stable kernels: keep primitives minimal, vet external dependencies.
- Adapter pattern for platform concerns: platform-specific code lives in `zaroxi-infrastructure-*` crates that implement trait contracts defined in core/domain.
- Intelligence and security are first-class families but avoid upward dependencies into application/interface layers.

---

## Layer model (concise)

- Kernel (`zaroxi-kernel-*`): canonical types, IDs, tiny traits.
- Core (`zaroxi-core-*`): engine/runtime primitives — buffers, render pipeline, input, scheduling.
- Domain (`zaroxi-domain-*`): pure business logic — workspace, buffer semantics, session lifecycle.
- Application (`zaroxi-application-*`): feature composition — workspace orchestration, editor composition, command routing.
- Interface (`zaroxi-interface-*`): concrete shells — desktop harness, CLI, lightweight app glue.

Special families:
- Infrastructure (`zaroxi-infrastructure-*`) — adapters for storage, RPC, OS integrations.
- Intelligence (`zaroxi-intelligence-*`) — agent runtimes, planning, context packing.
- Security (`zaroxi-security-*`) — policy, audit, sandbox helpers.

---

## Runtime flow (request / control / data)

1. User interaction at the Interface layer (desktop, CLI) produces high-level intents: open workspace, run command, request AI suggestion.
2. Interface maps intents to Application APIs (workspace orchestration, command dispatch). The application layer composes domain primitives and coordinates long-running flows.
3. Application executes domain operations (workspace model, buffer semantics). Domain crates encapsulate the pure logic and emit domain events.
4. Core crates implement low-level mechanics: in-memory buffers, transactions, checkpoints, render/view composition, scheduling and input dispatch. Core exposes trait contracts consumed by domain/application.
5. Infrastructure adapters (storage, RPC, OS) implement concrete persistence, transport, and platform-specific behavior behind well-defined traits.
6. Intelligence crates observe/consume domain/core state (via well-scoped interfaces) to provide planning, context packing, and suggestions. Intelligence must not perform side-effects directly; side-effects are requested through application-layer APIs for explicit approval.
7. Security crates provide policy evaluation, audit event models, and validation helpers. Enforcement occurs at the application/service boundary (explicit checks before privileged operations).

---

## Where infrastructure fits

- Infrastructure crates implement adapters for traits defined by core/domain. Examples: filesystem storage adapters, RPC transports, logging/tracing adapters.
- Infrastructure is responsible for side‑effects and platform integration; code here may depend on OS crates and external libraries.
- Keep protocol/format definitions in core/domain so infrastructure can provide multiple adapters without duplicating logic.

---

## Where intelligence fits

- Intelligence crates provide algorithmic, data‑processing, and planning capabilities: context packing, embeddings, agent planners, and verification helpers.
- Intelligence operates on copies or read-only views of domain/core state and returns suggestions, plans, or patches.
- All apply-side effects from intelligence must be mediated by application APIs and pass security/policy checks before applying to persistent state.

---

## Where security fits

- Security crates model policies, perform validation, and emit audit events; they are libraries used by application and infrastructure code.
- Runtime enforcement is implemented at the application boundary where requests are validated and audited.
- OS-level sandboxing and enterprise integrations are planned features (Phase 9+); current crates provide evaluation primitives and audit models.

---

## Desktop harness and GPU shell

- The desktop harness (`zaroxi-interface-desktop` + `zaroxi-interface-app`) is the active integration and test harness used to exercise workspace flows and runtime behaviors.
- GPU shell and render pipelines (core engine crates) are exercised by the harness; backend-specific adapters are feature-gated and isolated behind render-backend crates.
- The harness is the primary verification path for flows such as open/close, pending-close, checkpoint restore, command bar flows, and AI explanation/projection.

---

## Current state (pre‑Phase‑9)

- The repository implements a working Rust-native editor shell and desktop harness that composes interface → application → domain → core.
- Disk-backed persistence, hardened LSP, and end-to-end AI apply are intentionally not implemented yet.
- Infrastructure contains scaffolding and some adapters; production-grade integrations are a near-term priority (Phase 9).

---

> Note: References to older Tauri/iced UI experiments are historical. The canonical architecture uses the layer model above and does not rely on Tauri/iced as the current stack.

---

Related docs: see `docs/crates.md` for a crate-focused contribution guide and `docs/roadmap.md` for phase-based delivery checkpoints.
