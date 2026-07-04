# Architecture Decision Records

This directory records the significant architectural decisions behind Zaroxi
Studio. Each ADR captures one decision, the context that forced it, and its
consequences — so future contributors understand *why*, not just *what*.

## Format

We use a lightweight format (see [template.md](template.md)):

- **Status** — Proposed / Accepted / Superseded (by ADR-NNNN)
- **Context** — the forces and constraints
- **Decision** — what we chose
- **Consequences** — trade-offs, what it enables, what it costs

## Conventions

- Files are numbered and kebab-cased: `NNNN-short-title.md`.
- ADRs are append-only. To change a decision, add a new ADR and mark the old one
  *Superseded*.
- Keep them short — an ADR is a decision record, not a design doc.

## Index

| # | Decision | Status |
|---|---|---|
| [0001](0001-pure-rust-native-desktop-stack.md) | Pure-Rust native desktop stack | Accepted |
| [0002](0002-layered-crate-architecture.md) | Layered, crate-first architecture | Accepted |
| [0003](0003-unsafe-forbidden-with-documented-exceptions.md) | `unsafe` forbidden, with documented exceptions | Accepted |
| [0004](0004-committed-tree-sitter-grammar-runtime.md) | Pinned, per-platform Tree-sitter grammar runtime | Accepted |
| [0005](0005-ci-as-architecture-enforcement.md) | CI as architecture enforcement | Accepted |
