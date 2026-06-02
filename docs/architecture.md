# Architecture — System Responsibilities, Contracts, and Runtime Flow

Purpose: The authoritative architecture document for Zaroxi. Covers the layer model, dependency rules, concrete content/action contracts, shell geometry, and verification commands. This replaces older informal architecture notes.

---

## Layer model

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

Special families:
- **Infrastructure** (`zaroxi-infrastructure-*`) — adapters for storage, RPC, OS integrations.
- **Intelligence** (`zaroxi-intelligence-*`) — agent runtimes, planning, context packing.
- **Security** (`zaroxi-security-*`) — policy, audit, sandbox helpers.

## Dependencies

- Outer layers MAY depend on inner layers.
- Inner layers MUST NOT import outer layers.
- Application MUST NOT import `zaroxi-interface-*`.
- Domain MUST NOT import `zaroxi-interface-*` or `zaroxi-application-*`.
- Core MUST NOT import `zaroxi-domain-*` or `zaroxi-application-*` or `zaroxi-interface-*`.

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

## Traits in application-workspace

| Trait | Purpose | Methods |
|-------|---------|---------|
| `CloseContext` | Close-flow state | `latest_pending_close`, `set_pending_close`, `clear_pending_close`, `close_opened_buffer`, `set_status_message`, `set_close_result_status`, `clear_close_result_status`, `perform_session_close` |
| `CommandBarContext` | Command bar UI | `open_command_bar`, `close_command_bar`, `select_next_command`, `select_prev_command`, `latest_command_bar` |
| `RefreshContext` | Refresh + buffer/cursor | `has_pending_refresh_reason`, `set_pending_refresh_reason`, `active_buffer`, `latest_shell_context`, `perform_refresh` |

**DesktopComposition** implements all three traits. Add new trait methods there.

## Adding a new command

1. Add the label to `command_bar_labels()` in `workspace_view.rs`
2. Add a match arm in `execute_command_by_index()` in `workspace_view.rs`
3. If it uses new composition capabilities, extend the appropriate trait
4. Implement the new trait method on `DesktopComposition`

## Adding a new panel type

1. Define a content model in `zaroxi-core-engine-ui` (engine-owned) or `zaroxi-domain-*` (domain-specific)
2. Add a field to `ShellWorkContent`
3. Add assembly logic in `build_work_content()`
4. Wire the GPU draw path via `ShellWorkContent` field → draw function
5. Wire the transcript path via `render_chrome()`

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

## Runtime flow (request / control / data)

1. User interaction at the Interface layer (desktop, CLI) produces intents: open workspace, run command, request AI suggestion.
2. Interface maps intents to Application APIs (thin delegates calling shared functions on `DesktopComposition` as trait impl).
3. Application executes domain operations (workspace model, buffer semantics). Domain crates encapsulate pure logic and emit domain events.
4. Core crates implement low-level mechanics: in-memory buffers, transactions, checkpoints, render/view composition, scheduling and input dispatch.
5. Infrastructure adapters (storage, RPC, OS) implement concrete persistence, transport, and platform-specific behavior behind well-defined traits.
6. Intelligence crates observe/consume domain/core state to provide planning, context packing, and suggestions.
7. Security crates provide policy evaluation, audit event models, and validation helpers. Enforcement occurs at the application/service boundary.

## Where infrastructure fits

- Infrastructure crates implement adapters for traits defined by core/domain.
- Infrastructure is responsible for side-effects and platform integration.
- Keep protocol/format definitions in core/domain so infrastructure can provide multiple adapters.

## Where intelligence fits

- Intelligence crates provide planning, context packing, embeddings, and agent capabilities.
- Intelligence operates on copies or read-only views of domain/core state and returns suggestions, plans, or patches.
- All apply-side effects from intelligence are mediated by application APIs.

## Where security fits

- Security crates model policies, perform validation, and emit audit events.
- Runtime enforcement is at the application boundary.

## Current state (post-Phase-18)

- Architecture refactor phases 1–18 complete: desktop is a thin placement/render adapter.
- All shared orchestration lives in `zaroxi-application-workspace::workspace_view` via 3 traits.
- `ShellWorkContent` is the single content carrier for all panels.
- AI panel content flows inline through `build_work_content()` — no separate carrier.
- 47 desktop tests, 2 app-workspace unit, 9 architecture-contract tests all pass.
- Architecture check: 395 PASS, 0 FAIL.
- Disk-backed persistence, LSP, and full AI apply are intentionally not implemented yet.
- Desktop harness exercises runtime flows and GPU shell rendering.

## Verification

```bash
bash scripts/architecture_check.sh           # must PASS: 0 FAIL
cargo test -p zaroxi-interface-desktop        # all 47 tests green
cargo test -p zaroxi-application-workspace    # all tests green
cargo run -p zaroxi-interface-desktop --bin gui_shell  # transcript stable
```
