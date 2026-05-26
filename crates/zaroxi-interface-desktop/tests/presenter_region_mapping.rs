use zaroxi_interface_desktop::presenters::GpuShellPresenter;

#[test]
fn region_mapping_basic() {
    let width = 800;
    let height = 600;
    let chrome = 60;
    let status = 24;

    let regions = GpuShellPresenter::map_regions(width, height, chrome, status);

    // Debug-print actual mapped regions to aid diagnosis when running with --nocapture
    println!(
        "mapped regions -> chrome: x={} y={} w={} h={}; content: x={} y={} w={} h={}; status: x={} y={} w={} h={}",
        regions.chrome.x,
        regions.chrome.y,
        regions.chrome.width,
        regions.chrome.height,
        regions.content.x,
        regions.content.y,
        regions.content.width,
        regions.content.height,
        regions.status.x,
        regions.status.y,
        regions.status.width,
        regions.status.height,
    );

    // chrome at top (anchored to y=0)
    assert_eq!(regions.chrome.x, 0);
    assert_eq!(regions.chrome.y, 0);
    assert_eq!(regions.chrome.width, width);
    // The presenter may apply additional top inset adjustments; ensure chrome is a positive band
    // and it is at least the requested nominal chrome height.
    assert!(regions.chrome.height > 0);
    assert!(regions.chrome.height <= height);
    assert!(regions.chrome.height >= chrome);

    // status is an explicit bottom band of the requested height and anchored at the bottom
    assert_eq!(regions.status.x, 0);
    assert_eq!(regions.status.width, width);
    // status height is part of the public contract and should match the requested status band.
    assert_eq!(regions.status.height, status);
    assert_eq!(regions.status.y + regions.status.height, height);

    // content fills the middle between chrome and status
    assert_eq!(regions.content.x, 0);
    assert_eq!(regions.content.y, regions.chrome.y + regions.chrome.height);
    assert_eq!(regions.content.width, width);
    // content + chrome + status should cover the total height exactly
    assert_eq!(
        regions.content.y + regions.content.height + regions.status.height,
        height
    );

    // structural non-overlap invariants (chrome -> content -> status)
    assert!(regions.chrome.y + regions.chrome.height <= regions.content.y);
    assert!(regions.content.y + regions.content.height <= regions.status.y);
}

#[test]
fn paint_buffer_size_mismatch_is_noop() {
    let width = 64;
    let height = 64;
    let chrome = 8;
    let status = 4;

    let regions = GpuShellPresenter::map_regions(width, height, chrome, status);
    let mut buf = vec![0u8; (width as usize) * (height as usize) * 4 - 4]; // intentionally wrong size
    // Should not panic; paint_to_buffer silently returns when size mismatches.
    GpuShellPresenter::paint_to_buffer(width, height, &mut buf, &regions);
}

#[test]
fn paint_buffer_paints_regions() {
    // Use a modest framebuffer so the test runs quickly.
    let width = 128u32;
    let height = 64u32;
    let chrome = 10u32;
    let status = 6u32;

    let regions = GpuShellPresenter::map_regions(width, height, chrome, status);
    let mut buf = vec![0u8; (width as usize) * (height as usize) * 4];

    // Should paint without panic.
    GpuShellPresenter::paint_to_buffer(width, height, &mut buf, &regions);

    // Helper to read RGBA at (x,y)
    let read_pixel = |x: u32, y: u32| -> [u8; 4] {
        let idx = ((y * width + x) * 4) as usize;
        [buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]
    };

    // Chrome area sample (one row inside the chrome band)
    let chrome_sample_y = regions.chrome.y.saturating_add(1);
    let chrome_px = read_pixel(1, chrome_sample_y);
    assert_eq!(chrome_px, [32u8, 32u8, 40u8, 255u8]);

    // Content area sample (one row inside the content band)
    let content_sample_y = regions.content.y.saturating_add(1);
    let content_px = read_pixel(1, content_sample_y);
    assert_eq!(content_px, [220u8, 220u8, 225u8, 255u8]);

    // Status area sample (one row inside the status band, anchored to bottom)
    let status_sample_y = regions.status.y.saturating_add(regions.status.height.saturating_sub(1));
    let status_px = read_pixel(1, status_sample_y);
    // status region color
    assert_eq!(status_px, [48u8, 48u8, 56u8, 255u8]);
}
