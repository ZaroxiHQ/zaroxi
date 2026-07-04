# Runtime and Rendering

How the desktop shell runs and draws. This is the concrete companion to
[architecture.md](architecture.md) §5.

## The runnable app

- Binary: `gui_shell` in `zaroxi-interface-desktop`
  (`cargo run -p zaroxi-interface-desktop --bin gui_shell`).
- End-to-end wiring example / composition root: `apps/zaroxi-desktop-harness`.

## Rendering stack

The shell is drawn entirely with Rust GPU crates — no web view:

| Concern | Crate |
|---|---|
| Windowing / event loop | [winit](https://github.com/rust-windowing/winit) |
| GPU device / surface | [wgpu](https://github.com/gfx-rs/wgpu) |
| 2D vector rendering | [vello](https://github.com/linebender/vello) |
| Text shaping / layout | [cosmic-text](https://github.com/pop-os/cosmic-text) |

Engine composition (layout, compositor, fonts, draw) lives in the
`zaroxi-core-engine-*` crates; the desktop crate is a thin placement/draw layer
on top of shared logic.

## Event and content flow

The desktop composition builds a single content carrier that every panel reads
from, so the UI stays a placement/draw layer over shared orchestration.

```text
OS input (winit)
  → interface delegate (zaroxi-interface-desktop)
    → application use case (zaroxi-application-workspace)
      → domain / core operations
    → build_work_content()  ──► ShellWorkContent
                                  ├─ editor_body / editor_tabs
                                  ├─ explorer_items
                                  ├─ ai_panel_content
                                  └─ terminal_tabs
      GPU path        : ShellWorkContent → panel draw
      Transcript path : ShellWorkContent → text chrome (headless/CI)
```

`ShellWorkContent` is the **single content carrier** — panels never invent their
own carriers. Shared orchestration is exposed through a small set of traits in
`zaroxi-application-workspace` (close flow, command bar, refresh); the desktop
composition implements them, and desktop action files are thin delegates to the
shared functions.

## Headless / transcript mode

The shell can render a text transcript instead of a window. This is what CI and
non-GPU environments use to exercise runtime flows deterministically. It reads
the same `ShellWorkContent`, so the two paths cannot drift.

## Shell layout

The desktop shell uses a fixed region layout (widths are indicative):

```text
┌───────────────────────────────────────────────┐
│ toolbar                                         │
├──────┬───────────┬──────────────────┬──────────┤
│ rail │ sidebar   │ editor           │ AI panel │
│      │ (explorer)│  ├ tabs          │ (cockpit)│
│      │           │  ├ breadcrumb    │          │
│      │           │  ├ center editor │          │
│      │           │  └ terminal      │          │
├──────┴───────────┴──────────────────┴──────────┤
│ status bar (full width)                         │
└───────────────────────────────────────────────┘
```

## Syntax runtime

Syntax highlighting is provided by `zaroxi-core-platform-syntax` using
Tree-sitter. Compiled grammars are loaded at runtime from a per-platform runtime
directory (`runtime/treesitter/grammars/<os>-<arch>/`). Grammar revisions are
pinned, and the runtime is prepared with `tooling/scripts/prepare-treesitter.sh`.
See [testing-and-quality.md](testing-and-quality.md) and
[decisions/0004-committed-tree-sitter-grammar-runtime.md](decisions/0004-committed-tree-sitter-grammar-runtime.md).

## Where things live

| Concern | Crates |
|---|---|
| Window/event loop, shell composition | `zaroxi-interface-desktop` |
| Layout, compositor, fonts, draw | `zaroxi-core-engine-*` |
| Buffers, transactions, cursor, selection | `zaroxi-core-editor-*` |
| Syntax | `zaroxi-core-platform-syntax` |
| Orchestration / content assembly | `zaroxi-application-workspace` |
