# Testing and Quality

The checks that keep the workspace healthy, how they map to CI, and how to run
them locally. Architecture rules themselves are described in
[architecture.md](architecture.md) §7.

## CI workflows

Each workflow lives in `.github/workflows/`.

| Workflow | Purpose |
|---|---|
| `linux` / `macos` / `windows` | Build + test on each OS (Linux also runs fmt + clippy) |
| `architecture` | Hard gates: crate naming, circular deps, `architecture_check.sh`; advisory: layer matrix, crate size |
| `security-audit` | `cargo deny check` + `cargo audit` (push/PR + weekly) |
| `codeql` | CodeQL static analysis |
| `docs-link-check` | Validates Markdown links under `docs/` and the README |
| `release` / `release-drafter` / `changelog` | Build & attach binaries, draft notes, generate changelog |
| `labels` / `welcome-pr` / `docs-api-publish` | Repo automation and API docs |

## Quality gates

**Compilation & style** — `cargo fmt`, `cargo check --all-targets`,
`cargo clippy -- -D warnings` (default and `--all-features`), `cargo test`.

**Architecture** (hard gates):

- `check_circular_deps.py` — no dependency cycles.
- `check_crate_naming.py` — `zaroxi-<layer>-<name>` naming.
- `architecture_check.sh` — family-aware dependency direction with documented
  exceptions (composition roots; infra adapters implementing application ports).

**Architecture** (advisory, non-blocking): `check_layer_deps.py` (strict matrix
report) and `check_crate_size.py`.

**Security / supply chain** — `cargo deny check` (advisories, licenses, bans,
sources) and `cargo audit`. Justified, documented advisory exceptions live in
`deny.toml` and `.cargo/audit.toml`.

**`unsafe`** — forbidden workspace-wide; two documented exceptions
(`zaroxi-core-platform-syntax`, `zaroxi-core-workspace-files`). See
[decisions/0003-unsafe-forbidden-with-documented-exceptions.md](decisions/0003-unsafe-forbidden-with-documented-exceptions.md).

## Running the gates locally

Individual checks:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 .github/scripts/check_circular_deps.py
python3 .github/scripts/check_crate_naming.py
bash scripts/architecture_check.sh
```

Or run the full pipeline in one step:

- Linux/macOS — `tooling/scripts/run-ci-local.sh` (add `--fast` to skip clippy + link check)
- Windows — `pwsh -File tooling/scripts/run-ci-windows.ps1`

The local runners mirror CI order and print a pass/fail summary.

## Platform considerations

- **Linux** is the most exercised target. Building the GUI needs
  `libxkbcommon-dev` and `libwayland-dev` (installed by CI).
- **macOS / Windows** builds and tests run in CI but are less battle-tested.
- **Syntax tests** need platform grammars. Linux grammars are committed; other
  platforms build them via `tooling/scripts/prepare-treesitter.sh` before the
  syntax tests run. See [runtime-and-rendering.md](runtime-and-rendering.md).
- **`act`** (local GitHub Actions) is optional; the local runner scripts are the
  supported way to reproduce CI without Docker.

## Docs validation

`docs-link-check` validates Markdown links. To run it locally with the pinned
tool:

```bash
npx --yes markdown-link-check@3.11.2 -c .github/markdown-link-check.json README.md
```
