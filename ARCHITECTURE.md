# Architecture Contract

## Layers (inner → outer)

```
Kernel → Core → Domain → Application → Interface
```

| Layer | Crate examples | Owns |
|-------|---------------|------|
| **Kernel** | `zaroxi-kernel-types`, `zaroxi-kernel-math` | IDs, traits, math, shared primitives |
| **Core** | `zaroxi-core-engine-ui`, `zaroxi-core-engine-scene` | Engine composition (`ContentView`, `ShellWorkContent`, `compose_content_view`), scene primitives |
| **Domain** | `zaroxi-domain-ai`, `zaroxi-domain-session` | Stable value objects (`AiPanelContent`, `PendingClose`), serializable state |
| **Application** | `zaroxi-application-workspace`, `zaroxi-application-ai` | Orchestration traits (`CloseContext`, `CommandBarContext`, `RefreshContext`), shared DTOs (`workspace_view`), action functions, `build_work_content()` |
| **Interface** | `zaroxi-interface-desktop` | Winit event loop, GPU draw, shell layout, transcript, thin delegates to application action functions |

## Dependencies

- Outer layers MAY depend on inner layers
- Inner layers MUST NOT import outer layers
- Application MUST NOT import `zaroxi-interface-*`
- Domain MUST NOT import `zaroxi-interface-*` or `zaroxi-application-*`
- Core MUST NOT import `zaroxi-domain-*` or `zaroxi-application-*` or `zaroxi-interface-*`

Enforce with: `bash scripts/architecture_check.sh`

## Content flow

```
DesktopComposition::build_work_content()
  └─ reads metadata (ai_projection, active_buffer, visible_window, etc.)
  └─ delegates to application-workspace::build_work_content()
       └─ ShellWorkContent { editor_body, editor_tabs, explorer_items, ai_panel_content, terminal_tabs }
            ├─ GPU path: ShellFrame.work_content → panel::draw()
            └─ Transcript path: widgets::render_chrome(comp)
```

**`ShellWorkContent` is the single content carrier.** Every panel reads from it. No separate content carriers.

## Action flow

```
Desktop event → actions_command_bar.rs (thin delegate)
  └─ ws::execute_command_by_index(comp: &mut C, ...)
       └─ C: CommandBarContext + CloseContext + RefreshContext
       └─ all 8 commands handled in application-workspace
```

### Adding a new command

1. Add the label to `command_bar_labels()` in `workspace_view.rs`
2. Add a match arm in `execute_command_by_index()` in `workspace_view.rs`
3. If it uses new composition capabilities, extend the appropriate trait (`CloseContext`, `RefreshContext`, or `CommandBarContext`)
4. Implement the new trait method on `DesktopComposition`

### Adding a new panel type

1. Define a content model in `zaroxi-core-engine-ui` (if engine-owned) or `zaroxi-domain-*` (if domain-specific)
2. Add a field to `ShellWorkContent`
3. Add assembly logic in `build_work_content()`
4. Wire the GPU draw path via `ShellWorkContent` field → draw function
5. Wire the transcript path via `render_chrome()`

## Traits in application-workspace

| Trait | Purpose | Methods |
|-------|---------|---------|
| `CloseContext` | Close-flow state | `latest_pending_close`, `set_pending_close`, `clear_pending_close`, `close_opened_buffer`, etc. |
| `CommandBarContext` | Command bar UI | `open_command_bar`, `close_command_bar`, `select_next`, `select_prev`, `latest_command_bar` |
| `RefreshContext` | Refresh + buffer/cursor | `has_pending_refresh_reason`, `set_pending_refresh_reason`, `active_buffer`, `perform_refresh` |

**DesktopComposition** implements all three traits. Add new trait methods there.

## Shell geometry (do not change)

```
┌─────────────────────────────────────────────────┐
│ toolbar                                          │
├──────┬──────────────┬───────────────┬───────────┤
│ rail │ sidebar      │ editor        │ AI panel  │
│ 48px │ 256px        │ flex          │ 320px     │
│      │              ├───────────────┤           │
│      │              │ editor tabs   │           │
│      │              │ breadcrumb    │           │
│      │              │ center_editor │           │
│      │              │ center_bottom │           │
│      │              │  (terminal)   │           │
├──────┴──────────────┴───────────────┴───────────┤
│ status_bar (full width)                          │
└─────────────────────────────────────────────────┘
bottom_dock = 0px (hidden)
```

## Verification

```bash
bash scripts/architecture_check.sh  # must PASS: 0 FAIL
cargo test -p zaroxi-interface-desktop  # all 47 tests green
cargo test -p zaroxi-application-workspace  # all tests green
cargo run -p zaroxi-interface-desktop --bin gui_shell  # transcript stable
```
