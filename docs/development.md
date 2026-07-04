# Development

Local setup and the day-to-day loop. For what CI enforces, see
[testing-and-quality.md](testing-and-quality.md).

## Prerequisites

- A recent stable Rust toolchain (edition 2024).
- Linux GUI builds also need windowing headers:

  ```bash
  sudo apt-get install -y libxkbcommon-dev libwayland-dev
  ```

- Optional for the full local pipeline: `cargo install cargo-deny cargo-audit --locked`.

Bundled fonts are committed under `assets/fonts/`, so the shell runs without extra
downloads.

## Build and run

```bash
cargo build --workspace
cargo run -p zaroxi-interface-desktop --bin gui_shell
```

The first build compiles the dependency graph and can take a few minutes;
later builds are incremental.

## Test

```bash
cargo test --workspace                                   # everything
cargo test -p zaroxi-interface-desktop                    # one crate
cargo test -p zaroxi-core-platform-syntax --test highlight_spans
```

## Iterate quickly

```bash
cargo check -p <crate>                                    # fast type-check
cargo clippy --workspace --all-targets -- -D warnings     # lint as CI does
cargo fmt --all                                           # format
```

## Local CI helpers

| Script | Use it to |
|---|---|
| `tooling/scripts/run-ci-local.sh` | Run the full pipeline on Linux/macOS (`--fast` skips clippy + link check) |
| `tooling/scripts/run-ci-windows.ps1` | Run the pipeline on Windows (PowerShell) |
| `tooling/scripts/prepare-treesitter.sh` | Build/verify Tree-sitter grammars for the current platform |
| `tooling/scripts/verify-structure.sh` | Sanity-check the repo layout and that the workspace compiles |
| `scripts/architecture_check.sh` | Enforce the layer/dependency rules |

Run `run-ci-local.sh` before pushing to catch what CI would catch.

## Adding a crate

1. Create `crates/zaroxi-<layer>-<name>/` and pick the innermost layer that fits.
2. In its `Cargo.toml`, add `[lints]` with `workspace = true` and depend only on
   inner layers.
3. Add the crate to `[workspace].members` in the root `Cargo.toml`.
4. Run the naming, cycle, and architecture checks.

See [workspace-structure.md](workspace-structure.md) for placement rules.

## Debugging notes

- **Grammars missing / syntax not highlighting** — run
  `tooling/scripts/prepare-treesitter.sh`; it prints the resolved runtime dir.
- **Windowing errors on Linux** — confirm the `libxkbcommon-dev` /
  `libwayland-dev` packages are installed.
- **Architecture check failed** — the message names the offending edge; move the
  shared contract inward or introduce a port rather than depending outward.
