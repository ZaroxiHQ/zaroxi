use zaroxi_interface_desktop::presenters::GpuShellPresenter;

#[test]
fn region_mapping_basic() {
    let width = 800;
    let height = 600;
    let chrome = 60;
    let status = 24;

    let regions = GpuShellPresenter::map_regions(width, height, chrome, status);

    // chrome at top (anchored to y=0)
    assert_eq!(regions.chrome.x, 0);
    assert_eq!(regions.chrome.y, 0);
    assert_eq!(regions.chrome.width, width);
    // don't assert an exact chrome.height (top insets may change),
    // but ensure it's positive and fits within total height.
    assert!(regions.chrome.height > 0);
    assert!(regions.chrome.height <= height);

    // status is an explicit bottom band of the requested height and anchored at the bottom
    assert_eq!(regions.status.x, 0);
    assert_eq!(regions.status.width, width);
    assert_eq!(regions.status.height, status);
    assert_eq!(regions.status.y + regions.status.height, height);

    // content fills the middle between chrome and status
    assert_eq!(regions.content.x, 0);
    assert_eq!(regions.content.y, regions.chrome.y + regions.chrome.height);
    assert_eq!(regions.content.width, width);
    // content + chrome + status should cover the total height
    assert_eq!(
        regions.content.y + regions.content.height + regions.status.height,
        height
    );

    // structural non-overlap invariants
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

    // Chrome area sample (near the top)
    let chrome_px = read_pixel(1, 1);
    assert_eq!(chrome_px, [32u8, 32u8, 40u8, 255u8]);

    // Content area sample (just below chrome)
    let content_px = read_pixel(1, chrome + 1);
    assert_eq!(content_px, [220u8, 220u8, 225u8, 255u8]);

    // Status area sample (near the bottom)
    let status_y = height.saturating_sub(1).saturating_sub(1); // one row above bottom
    let status_px = read_pixel(1, status_y);
    // status region color
    assert_eq!(status_px, [48u8, 48u8, 56u8, 255u8]);
}
