AGENTS.md

Purpose
-------
This file documents conventions and operational guidance for automated agents that
work on this repository (CI bots, interactive coding agents, local dev automation,
etc.). Treat it as authoritative for style, build, and execution steps in the
worktree rooted at the repository root.

Use carefully: follow the repository-level rules and double-check any change that
mutates the codebase (commits, formatting, dependency changes).

Build / Lint / Test Commands
---------------------------
General (workspace) commands:
- Build workspace: `cargo build --workspace`
- Run all tests: `cargo test --workspace`
- Format all code: `cargo fmt --all`
- Lint (clippy) all crates: `cargo clippy --workspace --all-features -- -D warnings`

Build / run specific crates:
- Build a single crate: `cargo build -p <crate_name>`
- Run a specific binary: `cargo run -p <crate_name> --bin <binary_name>`
  Example GUI: `cargo run -p zaroxi-interface-desktop --bin gui_shell`

Feature-gated renderer (heavy GPU deps):
- Build with renderer features: `cargo build -p zaroxi-core-engine-render --features full_renderer`
- Run GUI with renderer feature: `cargo run -p zaroxi-interface-desktop --bin gui_shell --features full_renderer`

Running a single test
- From workspace root, run a single test in a crate:
  `cargo test -p <crate_name> <test_name>`
  Example: `cargo test -p zaroxi-core-engine-render test_rasterization`
- To see test output unbuffered (useful for println/eprintln in tests):
  `cargo test -p <crate_name> <test_name> -- --nocapture`
- For a test function using filters: `cargo test -p <crate_name> test_substring`

Running the UI in transcript/headless mode (CI-friendly):
- Some platform libs may not be available in CI. You can run the GUI in transcript
  mode which prints layout and logs instead of opening a window:
  `RUST_LOG=info cargo run -p zaroxi-interface-desktop --bin gui_shell`
- To enable text shader mask debug (diagnostics):
  `ZAROXI_TEXT_SHOW_MASK=1 RUST_LOG=info cargo run -p zaroxi-interface-desktop --bin gui_shell`

Code Style Guidelines
---------------------
Formatting and imports
- Always run `cargo fmt --all` before committing. The repository follows the
  Rust standard formatting. Agents should auto-format any modified files.
- Keep `use` imports grouped by crate (external crates first, workspace crates next
  or consistent with existing module ordering) and alphabetized where reasonable.
- Prefer explicit imports (`use crate::module::Type;`) over glob imports (`*`),
  except in test helpers or where the module provides a pre-defined re-export.

Types and naming
- Use full, descriptive names for variables and functions. Avoid single-letter
  variable names except in tight scopes like iterators (`i`, `j`) or short lambdas.
- Rust naming conventions:
  - Types, structs, enums, traits: `CamelCase` (eg. `InstanceSample`).
  - Functions, local variables: `snake_case` (eg. `prepare_buffer`).
  - Constants and static values: `UPPER_SNAKE_CASE` (eg. `DEFAULT_WIDTH`).
- Avoid Hungarian-style prefixes or cryptic abbreviations.
- Keep public API names stable and backwards-compatible where possible.

Error handling
- Use `Result<T, E>` for fallible functions. Prefer domain-specific error types
  (see `crate::error::RenderError`) over bare `anyhow` in library code.
- Bubble errors up with meaningful context using `thiserror`/`map_err`/`?`.
- Avoid silent `unwrap()` and `expect()` in library/core code. Tests and quick
  prototypes may use `unwrap()` but document it with a comment.
- When catching panics or performing defensive checks, log with `log::warn!` or
  `log::error!` with actionable messages.

Floating vs integer coordinates
- Keep layout math in floating point (`f32`) until the last possible step
  before rasterization or hardware upload. Avoid truncating to `i32` early; the
  cosmic/glyphon pipeline uses subpixel layout and metrics that must be preserved.

Concurrency and locking
- Use `Arc<Mutex<T>>` only where required by shared mutable state between threads.
- Minimize lock holdings; extract owned data from a borrow before calling into
  other components to avoid double mutable borrows.
- Prefer fine-grained locks for frequently-updated items; annotate why a particular
  mutex is required.

Logging
- Use `log::info!`, `log::debug!`, `log::warn!`, `log::error!` as appropriate.
- For diagnostic logs used by agents, prefer clearly prefixed lines such as
  `GUI_TEXT_GLYPH_RASTER:` or `GUI_TEXT_ATLAS_UPLOAD:` so tooling can easily
  grep them. Do not remove those diagnostic lines unless they are replaced by
  an improved diagnostic.

Tests
- Unit tests: keep them in the same module under `#[cfg(test)] mod tests { }`.
- Integration tests: place in `tests/` with descriptive names.
- When adding new behavior, include tests that exercise the behavior at the
  smallest scope that makes sense and include an integration test if it
  interacts across crates.

Documentation and comments
- Public items must have doc comments (`///`). Keep comments concise and
  explain why (not just what) for non-trivial logic.
- For patches that touch rendering or layout, add small inline comments explaining
  coordinate system choices (eg. physical vs logical pixels, baseline rules).

Repository and commit rules for agents
- NEVER commit changes that contain secrets (.env, keys) or expose private data.
- Do not run `git push --force`.
- Do not rewrite history on `main` branch. Create a PR/branch instead if push is required.
- Only create commits when explicitly requested by a human. When committing:
  - Use conventional commit message style: `type(scope): short summary`.
  - Keep commit bodies short and explain rationale for non-trivial changes.

AGENTS behavior & operational rules
----------------------------------
- Read-first: Always read the specific files you intend to modify before editing.
  Use the repo tools to locate files instead of blind searching.
- Use the `todowrite` tool to create a short, high-quality plan for any multi-step
  tasks. Update it as you progress and mark tasks `in_progress` and `completed`.
- Use the `edit` tool for atomic, minimal edits. Preserve original indentation
  and context when replacing strings.
- Avoid editing files outside the workspace root unless explicitly requested.
- Run `cargo check` or `cargo build` to validate changes locally before committing.
  Prefer quick checks limited to the crate you modified: e.g.,
  `cargo build -p <crate_name>`.
- When changes affect shaders, binaries, or platform code, consider running the
  full build with `--features full_renderer` to surface errors.

Special files and rules
- If `.cursorrules` or `.cursor/rules/` exist, follow those cursor-specific
  constraints; otherwise assume none apply.
- If `.github/copilot-instructions.md` or `.github/COPILOT.md` exists, respect
  the guidance for AI-assisted commits; check that file and include its rules in
  your work.
- If the repository contains an `AGENTS.md` closer to a file you will modify,
  prefer the most specific AGENTS.md (deeper in the tree) over this root file.

Performance & rendering notes
- For renderer code:
  - Prefer storing per-glyph data in float until the final vertex upload.
  - Use the CPU-side atlas as the single source-of-truth and perform a single
    `queue.write_texture` upload for atlas updates where supported.
  - Keep shader code strictly typed about texture format (if using R8 masks,
    sample `.r` explicitly) and add compile-time env gates for diagnostics.

Security & safety
- Do not execute external binaries downloaded from the web without approval.
- Do not add credentials to the repository. If a test requires secrets, document
  how to provide them via environment variables in CI only.

How to ask for help / signal uncertainty
- If you cannot proceed without more information, produce a short plan and
  ask the repository owner or reviewer for clarification before making changes.
- When opening a PR, include a concise description of what you changed, why,
  and how it was validated (which cargo checks were run, which tests passed).

Appendix: Quick commands summary
- Full build: `cargo build --workspace`
- Single crate build: `cargo build -p <crate>`
- Run GUI: `cargo run -p zaroxi-interface-desktop --bin gui_shell`
- Run GUI (mask debug): `ZAROXI_TEXT_SHOW_MASK=1 cargo run -p zaroxi-interface-desktop --bin gui_shell`
- Single test: `cargo test -p <crate> <test_name> -- --nocapture`
- Format: `cargo fmt --all`
- Lint: `cargo clippy --workspace --all-features -- -D warnings`

If you are an agent editing code in this repo, include a short summary of what
you intend to change as a preamble comment before performing edits, and run the
relevant `cargo check` command after edits. Follow the commit rules above.

-- End of AGENTS.md
