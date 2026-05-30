fn main() {
    // Simple headless smoke test for the desktop cosmic shim.

    use zaroxi_interface_desktop::text;
    use zaroxi_interface_desktop::text::cosmic_text_renderer;

    // Initialize renderer
    if let Err(e) = text::init_cosmic_renderer() {
        eprintln!("init_cosmic_renderer failed: {}", e);
        std::process::exit(1);
    }

    let renderer = text::COSMIC_RENDERER.get().expect("COSMIC_RENDERER not set").clone();

    // Small framebuffer
    let fb_w = 400u32;
    let fb_h = 120u32;
    let mut buffer = vec![255u8; (fb_w * fb_h * 4) as usize];

    // Draw some text
    let res = cosmic_text_renderer::CosmicTextRenderer::draw_text(
        &renderer,
        &mut buffer,
        fb_w,
        fb_h,
        10,
        20,
        "The quick brown fox jumps over the lazy dog",
        [0, 0, 0, 255],
        Some(300),
    );

    if let Err(e) = res {
        eprintln!("draw_text returned error: {}", e);
        std::process::exit(2);
    }

    eprintln!("draw_text completed; buffer len={}", buffer.len());
}
