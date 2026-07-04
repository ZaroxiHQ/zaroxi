# 0003 — `unsafe` forbidden, with documented exceptions

- **Status:** Accepted
- **Date:** 2026-07-04

## Context

Memory-safety is a core reason to build this in Rust. Unconstrained `unsafe`
erodes that guarantee and is hard to audit as the codebase grows. A few
capabilities, however, genuinely require `unsafe` (loading shared libraries via
FFI, memory-mapping files).

## Decision

Set `unsafe_code = "forbid"` at the workspace level, inherited by every crate via
`[lints] workspace = true`. Grant exactly two documented exceptions where FFI/OS
operations are unavoidable:

- `zaroxi-core-platform-syntax` — dynamic Tree-sitter grammar loading (`libloading`).
- `zaroxi-core-workspace-files` — memory-mapping large files (`memmap2`).

Each exception crate carries a crate-level `#![allow(unsafe_code)]` with a
rationale, and every `unsafe` block has a `// SAFETY:` note.

## Consequences

- **Enables:** an `unsafe`-free default across ~145 crates and a tiny, explicitly
  reviewed surface where it is needed.
- **Costs:** the two exception crates opt out of the workspace lint and must be
  reviewed with extra care; new `unsafe` needs an ADR/exception, not a local
  `allow`.
