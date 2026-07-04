# Workspace Structure

How the monorepo is laid out and how to place new code. For layer
*responsibilities* and dependency rules, see [architecture.md](architecture.md).

## Top-level layout

```text
apps/          # runnable binaries (composition roots)
crates/        # ~145 layered library crates
docs/          # documentation set
tooling/       # local CI + setup helpers
  scripts/     #   run-ci-local.sh, run-ci-windows.ps1, prepare-treesitter.sh, verify-structure.sh
scripts/       # repo scripts (architecture_check.sh, packaging, generators)
.github/       # workflows, issue templates, architecture checkers
assets/        # bundled runtime assets (fonts)
```

## Crate families

Every library crate is named `zaroxi-<layer>-<name>`. Counts are approximate and
move as crates are added.

| Family | Prefix | Count | Role |
|---|---|---:|---|
| Kernel | `zaroxi-kernel-` | 12 | Primitives: ids, errors, time, math, traits |
| Core | `zaroxi-core-` | 79 | Editor + rendering engine, syntax, platform integration |
| Domain | `zaroxi-domain-` | 8 | Value objects and models |
| Application | `zaroxi-application-` | 12 | Orchestration, use cases, ports |
| Interface | `zaroxi-interface-` | 5 | Desktop shell, CLI, widgets, theme |
| Infrastructure | `zaroxi-infrastructure-` | 14 | Adapters: storage, rpc, network, memory |
| Intelligence | `zaroxi-intelligence-` | 9 | Agents, planning, context, memory, tools, safety |
| Security | `zaroxi-security-` | 6 | Policy, audit, validation, crypto, sandbox |

The Core family is large and sub-grouped by concern, e.g.
`zaroxi-core-editor-*` (buffer, cursor, selection, transaction, history…),
`zaroxi-core-engine-*` (ui, render, layout, font, compositor…), and
`zaroxi-core-platform-*` (syntax, git, lsp, terminal…).

## Naming rules (enforced)

`.github/scripts/check_crate_naming.py` enforces:

- Library crates under `crates/` must be `zaroxi-<layer>-<name>` with a valid
  layer. Grouped core subsystems require a concern
  (`zaroxi-core-editor-buffer`); flat core concerns are terminal
  (`zaroxi-core-io`).
- `apps/` are composition roots: they only need the `zaroxi-` prefix and are not
  bound to a layer name (e.g. `zaroxi-desktop-harness`).
- `tooling/` and `docs/` are not library crates and are excluded.

## apps vs crates vs tooling

- **`crates/`** — libraries that obey the strict layer direction.
- **`apps/`** — composition roots; the only place allowed to combine many layers
  and start the runtime. `apps/zaroxi-desktop-harness` wires application services
  to infrastructure adapters.
- **`tooling/` and `scripts/`** — developer/CI helpers, not part of the shipped
  dependency graph.

## Placing new code

- New library? Pick the innermost layer that fits, name it
  `zaroxi-<layer>-<name>`, add it to `[workspace].members` in the root
  `Cargo.toml` with `[lints] workspace = true`, and depend only inward.
- Need something from an outer layer? Don't reach up — move the shared contract
  down (Core/Domain) or define a **port** in Application and implement it in
  Infrastructure.
- New runnable binary or end-to-end wiring? That belongs in `apps/`.

Verify placement locally with `scripts/architecture_check.sh` and the naming/cycle
checks; see [testing-and-quality.md](testing-and-quality.md).
