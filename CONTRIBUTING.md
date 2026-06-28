# Contributing to Zaroxi Studio

Thank you for your interest in contributing to Zaroxi Studio! This document contains the canonical contribution workflow plus the strict layered architecture and dependency rules that must be followed when adding new crates or changing cross-crate dependencies.

Table of contents
- Layer rules and responsibilities
- Dependency direction rules (enforced by CI)
- How to create a new crate (use `just new-crate`)
- How CI works
- Local developer shortcuts (just commands)
- How to extend the layer/namespace rules

---

## Layer rules and responsibilities

Zaroxi organizes code into the following namespaces (prefixes are used for crate names):

- kernel: `zaroxi-kernel-*`
  - Responsibilities: low-level primitives, ID types, small math helpers, memory, traits that are core to the runtime.
  - Dependency allowance: may only depend on other kernel crates and external third-party crates.

- core: `zaroxi-core-*`
  - Responsibilities: engine, runtime, core services that build on kernel primitives.
  - Dependency allowance: may only depend on kernel crates and other core crates.

- domain: `zaroxi-domain-*`
  - Responsibilities: domain models, business logic that uses core primitives.
  - Dependency allowance: may only depend on kernel and core crates.

- application: `zaroxi-application-*`
  - Responsibilities: application-level orchestration, app features that use domain APIs.
  - Dependency allowance: may only depend on kernel, core and domain crates.

- interface: `zaroxi-interface-*`
  - Responsibilities: UI, CLI, desktop/IDE front-ends.
  - Dependency allowance: can depend on application, domain, core and kernel (anything below it).

Top-level special namespaces:
- intelligence: `zaroxi-intelligence-*`
  - Responsibilities: AI/ML features and planners.
  - Dependency allowance: may depend on kernel, core and domain only.
- security: `zaroxi-security-*`
  - Responsibilities: audit, crypto, policy.
  - Dependency allowance: may depend on kernel, core and domain only.
- infrastructure: `zaroxi-infrastructure-*`
  - Responsibilities: adapters (networking, containers, ssh, storage).
  - Dependency allowance: may depend on kernel and core only.

Important: No circular dependencies are allowed. The CI enforces the layer rules automatically.

---

## Dependency direction rules (examples)

Allowed:
- `zaroxi-core-graphics` -> depends on `zaroxi-kernel-math`
- `zaroxi-domain-workspace` -> depends on `zaroxi-core-runtime`
- `zaroxi-application-editor` -> depends on `zaroxi-domain-buffer` and `zaroxi-core-engine`

Forbidden:
- `zaroxi-core-foo` -> depends on `zaroxi-domain-bar` (core must not depend on domain)
- `zaroxi-domain-x` -> depends on `zaroxi-application-y` (domain must not depend on application)
- `zaroxi-infrastructure-net` -> depends on `zaroxi-application-search` (infrastructure must only depend on kernel/core)
- Circular: A -> B -> A (never allowed)

If you need an exception to the rules (rare), open an issue describing the use case and get approval from maintainers. Once approved, CI can be updated to include an explicit exception.

---

## How to create a new crate

We provide a helper: `just new-crate NAME`

This:
- scaffolds `crates/NAME` with a starter Cargo.toml and src/lib.rs
- registers the crate under the workspace members in the root `Cargo.toml`
- uses the appropriate crate name you provide (please follow namespace naming rules)

Example:
- `just new-crate zaroxi-core-logger`

After creating the crate:
- Implement your code and tests
- Run `just check` locally to ensure linting/formatting pass
- Run `just test` to run tests

The `just` commands are documented in the Local developer shortcuts section.

---

## How CI works (high level)

CI runs on push and pull_request to the `main` branch and performs the following jobs:

- check
  - Runs `cargo check --workspace --all-targets` to ensure the project compiles.
- clippy
  - Runs `cargo clippy --workspace --all-targets -- -D warnings` (treat warnings as errors).
- test
  - Runs `cargo test --workspace`.
- fmt
  - Runs `cargo fmt --all -- --check` to ensure formatting.
- deny
  - Runs `cargo deny check` using the root `deny.toml` policy that enforces:
    - license policy (allowed/forbidden licenses)
    - known advisories
    - duplicate dependency versions
    - other workspace-wide bans
- deps
  - Runs a custom script that parses workspace crate manifests and ensures cross-crate dependency directions follow the layer rules described above.

If any job fails, CI fails. Fix the problem and push again. Use `just ci` to run all checks locally before pushing.

---

## Local developer shortcuts (just commands)

We provide a `justfile` with convenient recipes. Common commands:

- `just check` — clippy + fmt check
- `just test` — run all tests
- `just deny` — run `cargo deny check`
- `just ci` — run all checks locally (check, clippy, fmt, test, deny, deps)
- `just new-crate NAME` — scaffold and register a new crate

Quick commands to run locally (example):
- `just check`
- `just test`
- `just ci`

These commands are intentionally thin wrappers around the canonical cargo/rust tooling.

---

## License policy enforced by cargo-deny

The workspace policy permits the following licenses:
- MIT
- Apache-2.0
- BSD-2-Clause
- BSD-3-Clause
- ISC
- Unicode-DFS-2016

The following licenses are explicitly forbidden for workspace dependencies:
- GPL-2.0
- GPL-3.0
- AGPL-3.0

The deny policy also flags duplicate versions of the same crate where possible and known advisories.

---

## How to extend layer/namespace rules

- The dependency-direction rules are defined in `.github/scripts/check_layer_deps.py` and are invoked by CI.
- To add new namespaces or update rules:
  1. Edit `.github/scripts/check_layer_deps.py` and add the prefix → layer mapping or update the allowed-dependencies table.
  2. Add tests if necessary.
  3. Update `CONTRIBUTING.md` to reflect the new rule.
  4. Open a PR to apply the change.

We intentionally keep the enforcement script human-readable and self-contained so maintainers can adapt rules as the project evolves.

---

## Notes and best practices

- Keep your crates small and focused.
- Prefer composing functionality via well-defined public APIs instead of reaching across layers.
- Add unit tests for logic and integration tests for public module interactions.
- If you need help deciding where a new crate belongs, open an issue describing the responsibilities and proposed name.

Thank you for contributing to Zaroxi Studio! 🎉