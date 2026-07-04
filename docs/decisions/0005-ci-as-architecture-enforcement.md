# 0005 — CI as architecture enforcement

- **Status:** Accepted
- **Date:** 2026-07-04

## Context

Layer boundaries and naming rules are only real if they are enforced. Relying on
review alone lets the dependency graph rot as the project grows across ~145
crates.

## Decision

Encode the architecture as executable checks that gate merges, in the
`Architecture` workflow:

- `check_circular_deps.py` — no dependency cycles (hard gate).
- `check_crate_naming.py` — `zaroxi-<layer>-<name>` naming (hard gate).
- `architecture_check.sh` — family-aware dependency direction with two documented
  exceptions: composition roots (`apps/`) may cross layers, and infrastructure
  adapters may depend on the application crate whose port they implement.
- `check_layer_deps.py` and `check_crate_size.py` — advisory reports (non-blocking).

Supply-chain and safety are enforced alongside via `cargo deny`, `cargo audit`,
CodeQL, and the `unsafe` policy ([ADR-0003](0003-unsafe-forbidden-with-documented-exceptions.md)).

## Consequences

- **Enables:** boundaries that stay honest automatically, and a clear signal when
  a change violates the architecture.
- **Costs:** the checks must be maintained as the model evolves; deliberate
  exceptions must be encoded and documented rather than ignored.
- Reproduce locally with `tooling/scripts/run-ci-local.sh`; see
  [testing-and-quality.md](../testing-and-quality.md).
