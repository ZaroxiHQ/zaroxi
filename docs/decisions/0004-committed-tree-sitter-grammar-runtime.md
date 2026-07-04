# 0004 — Pinned, per-platform Tree-sitter grammar runtime

- **Status:** Accepted
- **Date:** 2026-07-04

## Context

Syntax highlighting uses Tree-sitter, which needs compiled grammar libraries at
runtime. Building grammars from floating upstream branches made CI
non-reproducible: grammar node names drifted from the committed highlight queries,
and some grammars added external scanners that broke `dlopen`. Grammars are also
platform-specific (`.so` / `.dylib` / `.dll`).

## Decision

- **Pin** every grammar to a fixed upstream revision in the registry.
- **Commit** the Linux (`linux-x86_64`) grammar libraries so Linux CI and most
  contributors work with zero build/network steps.
- **Build per platform** on demand: the loader resolves
  `runtime/treesitter/grammars/<os>-<arch>/`, and
  `tooling/scripts/prepare-treesitter.sh` builds the current platform's grammars
  (auto-detecting external scanners) for macOS/Windows.

## Consequences

- **Enables:** reproducible, offline-friendly syntax highlighting on Linux and
  deterministic builds elsewhere.
- **Costs:** committed binary artifacts for Linux, and macOS/Windows require a
  grammar build step (best-effort, run before syntax tests).
- See [runtime-and-rendering.md](../runtime-and-rendering.md).
