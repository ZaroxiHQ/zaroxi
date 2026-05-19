/*!
Feature-gated native GPU shell bootstrap.

This binary provides two builds:
- Non-feature (default): prints a compatibility message and exits.
- With feature "gpu_shell_bin": runs a minimal native window using minifb,
  reusing the existing GpuShellPresenter for region mapping and painting.

Design constraints:
- Tiny, minimal event loop with one window and one presenter.
- All native deps are optional and enabled only when the feature is requested.
- No duplication of composition logic: we obtain a real shell-facing view model
  and hand it to a small adapter which delegates to the presenter's pure mapping.
*/

#[cfg(not(feature = "gpu_shell_bin"))]
fn main() {
    eprintln!("gpu_shell: native GPU shell is not started in this build.");
    eprintln!("If you intended to run the native windowed demo, enable the feature:");
    eprintln!("  cargo run -p zaroxi-interface-desktop --bin gpu_shell --features=\"gpu_shell_bin\"");
}

#[cfg(feature = "gpu_shell_bin")]
fn main() {
    // Minimal, robust native path using minifb.
    use std::{thread::sleep, time::Duration};
    use minifb::{Key, Window, WindowOptions};

    use zaroxi_interface_desktop::presenters::gpu_shell::GpuShellPresenter;
    use zaroxi_interface_desktop::gpu_shell_adapter::{view_model_to_regions_from_scratch, NativeKey, map_native_to_ui_event};
    use zaroxi_interface_desktop::events::{EventBridge, ActionExecutor, Action};

    // Local, tiny ActionExecutor that forwards actions into a local UI-only handler.
    // This executor intentionally delegates to the crate-local runtime helper which
    // calls the existing action/refresh path and returns an adapted ShellRegions.
    struct LocalExecutor {
        width: u32,
        height: u32,
        regions: zaroxi_interface_desktop::presenters::gpu_shell::ShellRegions,
    }

    impl LocalExecutor {
        fn new(width: u32, height: u32) -> Self {
            // Start with no active buffer marker; the executor will update regions
            // by applying actions via the canonical runtime helper.
            let regions = view_model_to_regions_from_scratch(width, height, None);
            Self { width, height, regions }
        }

        fn current_regions(&self) -> &zaroxi_interface_desktop::presenters::gpu_shell::ShellRegions {
            &self.regions
        }
    }

    impl ActionExecutor for LocalExecutor {
        fn execute(&mut self, action: Action) {
            // Delegate to the crate-local runtime helper which performs:
            // event -> action -> existing action layer invocation -> refresh -> adapt -> ShellRegions
            let updated = zaroxi_interface_desktop::gpu_shell_runtime::apply_action_and_get_regions(action, self.width, self.height);
            self.regions = updated;
        }
    }

    let width: u32 = 800;
    let height: u32 = 600;

    // RGBA8 backing buffer for the presenter (mutated in place).
    let mut rgba_buf = vec![0u8; (width as usize) * (height as usize) * 4];
    // minifb expects a u32 per pixel buffer (format: 0x00RRGGBB little-endian).
    let mut pixel_buf: Vec<u32> = vec![0u32; (width as usize) * (height as usize)];

    // Initialize executor (it holds the current adapted regions and updates them
    // when actions are executed).
    let mut executor = LocalExecutor::new(width, height);

    let mut window = Window::new(
        "Zaroxi - GPU Shell (minimal)",
        width as usize,
        height as usize,
        WindowOptions::default(),
    )
    .expect("Failed to create window");

    // Simple render loop driven by an up-to-date ShellRenderViewModel.
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // POLL INPUT: translate native keys into UiEvent via the shared adapter helper
        // and route through the existing EventBridge. This keeps mapping logic in one place.
        //
        // Minimal set of keys:
        // - Up, Down, Enter, A (character example)
        if window.is_key_down(Key::Up) {
            if let Some(ev) = map_native_to_ui_event(NativeKey::Up) {
                EventBridge::handle_event(ev, &mut executor);
            }
        }
        if window.is_key_down(Key::Down) {
            if let Some(ev) = map_native_to_ui_event(NativeKey::Down) {
                EventBridge::handle_event(ev, &mut executor);
            }
        }
        if window.is_key_down(Key::Enter) {
            if let Some(ev) = map_native_to_ui_event(NativeKey::Enter) {
                EventBridge::handle_event(ev, &mut executor);
            }
        }
        if window.is_key_down(Key::A) {
            if let Some(ev) = map_native_to_ui_event(NativeKey::Char('a')) {
                EventBridge::handle_event(ev, &mut executor);
            }
        }

        // Paint into the RGBA buffer using the presenter's pure function and the
        // most-recent adapted regions held by the executor.
        GpuShellPresenter::paint_to_buffer(width, height, &mut rgba_buf, executor.current_regions());

        // Convert RGBA8 -> 0x00RRGGBB u32 pixels (drop alpha).
        for i in 0..(width as usize * height as usize) {
            let base = i * 4;
            let r = rgba_buf[base] as u32;
            let g = rgba_buf[base + 1] as u32;
            let b = rgba_buf[base + 2] as u32;
            pixel_buf[i] = (r << 16) | (g << 8) | b;
        }

        // Update window; ignore occasional update errors for robustness.
        let _ = window.update_with_buffer(&pixel_buf, width as usize, height as usize);

        // Throttle to a modest refresh rate; real apps will hook into the composition tick.
        sleep(Duration::from_millis(100));
    }
}
