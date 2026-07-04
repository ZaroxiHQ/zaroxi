# 0002 — Layered, crate-first architecture

- **Status:** Accepted
- **Date:** 2026-07-04

## Context

An IDE runtime accumulates large, tangled modules if left as a few big crates.
We want the editor engine, orchestration, and UI to evolve independently, keep
compile units small, and make dependency direction explicit and checkable.

## Decision

Organize the workspace as many small crates in strict layers
(**Kernel → Core → Domain → Application → Interface**) plus cross-cutting
**Infrastructure**, **Intelligence**, and **Security** families. Dependencies
point inward. The application layer defines ports; infrastructure implements
them; composition roots (`apps/`) wire them together.

Naming is fixed: `zaroxi-<layer>-<name>`.

## Consequences

- **Enables:** small blast radius, parallel compilation, testable units, and
  mechanically enforceable boundaries (see [ADR-0005](0005-ci-as-architecture-enforcement.md)).
- **Costs:** more manifests and more up-front thought about where code belongs;
  cross-layer needs must be resolved by moving contracts down or adding a port,
  not by reaching up.
- See [architecture.md](../architecture.md) and
  [workspace-structure.md](../workspace-structure.md).
