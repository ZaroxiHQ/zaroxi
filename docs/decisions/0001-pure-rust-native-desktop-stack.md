# 0001 — Pure-Rust native desktop stack

- **Status:** Accepted
- **Date:** 2026-07-04

## Context

Most editors ship on a browser engine (Electron) or a webview wrapper (Tauri).
That brings a large runtime, a JavaScript layer, and limited control over
rendering and input latency. Zaroxi Studio targets a native editor with full
control over the frame and a dependency-light runtime.

## Decision

Build the desktop shell entirely in Rust on a native GPU stack:

- [winit](https://github.com/rust-windowing/winit) — windowing and events
- [wgpu](https://github.com/gfx-rs/wgpu) — GPU device/surface
- [vello](https://github.com/linebender/vello) — 2D vector rendering
- [cosmic-text](https://github.com/pop-os/cosmic-text) — text shaping/layout

No web view, JavaScript runtime, or Electron/Tauri layer.

## Consequences

- **Enables:** a single native process, direct control over rendering and input,
  and a small deployable footprint.
- **Costs:** more to build than reusing a browser (UI primitives, text layout,
  compositing are our responsibility), and dependence on the maturity of the Rust
  GPU ecosystem across platforms.
- **Revisit if:** a platform's GPU backend proves unviable for a target audience.
